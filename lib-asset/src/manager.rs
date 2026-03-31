use core::fmt;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{FsResolver, PrefabFactory};
use mimiq::image;
use mimiq::{FileReady, FsServer};

use anyhow::Context;
use hashbrown::{HashMap, HashSet};
use hecs::{BuiltEntityClone, EntityBuilderClone};
use tracing::{instrument, warn};

const TARGET_NAME: &str = "asset_manager";

#[cfg(feature = "dbg")]
pub struct AssetNodeDebug<'a> {
    pub path: &'a Path,
    pub ty: &'static str,
    pub state: &'static str,
    pub deps_not_loaded: usize,
}

pub struct AssetManager<T> {
    pub fs_resolver: FsResolver,
    prefab_factory: Rc<PrefabFactory<T>>,
    fs_server: Rc<dyn FsServer>,
    nodes: HashMap<Rc<Path>, AssetNode<T>>,
    dependents: HashMap<Rc<Path>, HashSet<Rc<Path>>>,
    queue: VecDeque<Rc<Path>>,
}

impl<T: 'static> AssetManager<T> {
    pub fn new(fs_server: Rc<dyn FsServer>, prefab_factory: PrefabFactory<T>) -> Self {
        AssetManager {
            fs_server,
            prefab_factory: Rc::new(prefab_factory),
            fs_resolver: FsResolver::new(),
            nodes: HashMap::new(),
            dependents: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    #[cfg(feature = "dbg")]
    pub fn iter_node_dependents(&self) -> impl Iterator<Item = (&Path, &HashSet<Rc<Path>>)> {
        self.dependents.iter().map(|(k, v)| (&**k, v))
    }

    #[cfg(feature = "dbg")]
    pub fn iter_node_debug(&'_ self) -> impl Iterator<Item = AssetNodeDebug<'_>> {
        self.nodes.values().map(|x| x.dbg_info())
    }

    pub fn iter_assets_to_load(&self) -> impl Iterator<Item = &Rc<Path>> {
        self.dependents
            .keys()
            .filter(|x| self.nodes.get(*x).is_none())
    }

    pub fn is_loaded(&self, path: impl AsRef<Path>) -> bool {
        self.nodes
            .get(path.as_ref())
            .map(|x| x.state.is_initialized())
            .unwrap_or_default()
    }

    pub fn load_prefab(
        &mut self,
        src: impl AsRef<Path>,
        callback: impl FnOnce(&mut T, &FsResolver, BuiltEntityClone, &Path) + 'static,
    ) {
        let factory_deps = self.prefab_factory.clone();
        let factory_finish = self.prefab_factory.clone();
        let path: Rc<Path> = src.as_ref().into();
        let path_borrow = path.clone();

        let node = AssetNode::new(
            path.clone(),
            "prefab",
            Box::new(move |bytes| {
                let pre_prefab = serde_json::from_slice(bytes).context("deser prefab")?;
                factory_deps.list_deps(&pre_prefab).context("list deps")
            }),
            Box::new(move |ctx, res, data| {
                let pre_prefab = serde_json::from_slice(&data).context("deser prefab")?;
                let mut builder = EntityBuilderClone::new();
                factory_finish
                    .build(ctx, &mut builder, &pre_prefab)
                    .context("build prefab")?;
                callback(ctx, res, builder.build(), &path_borrow);
                Ok(())
            }),
        );
        self.create_asset_task(node);
    }

    pub fn load_image(
        &mut self,
        src: impl AsRef<Path>,
        callback: impl FnOnce(&mut T, &FsResolver, image::DynamicImage, &Path) + 'static,
    ) {
        let path: Rc<Path> = src.as_ref().into();
        let path_borrow = path.clone();

        let node = AssetNode::new(
            path.clone(),
            "image",
            Box::new(|_| Ok(Vec::new())),
            Box::new(move |ctx, res, data| {
                let img = image::load_from_memory(&data).context("decode img")?;
                callback(ctx, res, img, &path_borrow);
                Ok(())
            }),
        );
        self.create_asset_task(node);
    }

    fn create_asset_task(&mut self, node: AssetNode<T>) {
        let path = node.src.clone();
        if self.nodes.contains_key(&path) {
            tracing::debug!(?path, "Will not queue. Already have a node");
            return;
        }

        self.nodes.insert(path.clone(), node);
        self.fs_server.load_file(&path);
    }

    #[instrument(target=TARGET_NAME, skip_all, fields(path=?event.path))]
    pub fn on_file_ready(&mut self, ctx: &mut T, event: FileReady) -> anyhow::Result<()> {
        let Some(start) = self.node_file_ready(event)? else {
            return Ok(());
        };

        self.queue.push_back(start);
        while let Some(asset) = self.queue.pop_front() {
            let mut node = self
                .nodes
                .remove(&asset)
                .expect("BUG: traversed to a non-existent node");
            node = node.dependency_ready(&self.fs_resolver, ctx)?;
            let ready = node.state.is_initialized();
            self.nodes.insert(asset.clone(), node);

            if !ready {
                continue;
            }

            let Some(tonotify) = self.dependents.get(&asset) else {
                // It's okay to have no dependents entry, because dependents insert it.
                continue;
            };
            self.queue.extend(tonotify.iter().cloned());
        }

        Ok(())
    }

    fn node_file_ready(&mut self, event: FileReady) -> anyhow::Result<Option<Rc<Path>>> {
        let asset_path = Rc::<Path>::from(event.path);
        let Some(mut node) = self.nodes.remove(&asset_path) else {
            warn!("no such node: {asset_path:?}");
            return Ok(None);
        };
        let data = event
            .bytes_result
            .with_context(|| format!("load_file({asset_path:?})"))?;
        let deps: Vec<PathBuf>;
        let already_ready: bool;
        (deps, already_ready, node) = node.bytes_ready(&self.nodes, data)?;
        tracing::debug!(target: TARGET_NAME, deps=?deps, path=?asset_path, state=?node.state, "bytes parsed");
        self.nodes.insert(asset_path.clone(), node);

        for path in deps {
            self.dependents
                .entry(path.into())
                .or_default()
                .insert(asset_path.clone());
        }

        Ok(already_ready.then(|| asset_path.clone()))
    }
}

struct AssetNode<T> {
    src: Rc<Path>,
    ty: &'static str,
    state: AssetNodeState<T>,
}

impl<T> AssetNode<T> {
    #[cfg(feature = "dbg")]
    fn dbg_info(&'_ self) -> AssetNodeDebug<'_> {
        let (state, deps_not_loaded) = match self.state {
            AssetNodeState::PendingFsRequest { .. } => ("PendingFs", 0),
            AssetNodeState::BytesReady { deps_not_loaded, .. } => ("BytesReady", deps_not_loaded),
            AssetNodeState::Initialized => ("Initialized", 0),
        };
        AssetNodeDebug { path: &*self.src, ty: self.ty, state, deps_not_loaded }
    }

    fn new(
        src: Rc<Path>,
        ty: &'static str,
        on_bytes_ready: OnBytesReady,
        on_deps_ready: OnDepsReady<T>,
    ) -> Self {
        let state = AssetNodeState::PendingFsRequest { on_bytes_ready, on_deps_ready };
        AssetNode { src, ty, state }
    }

    #[instrument(target=TARGET_NAME, skip_all, fields(path=?self.src, ty=self.ty))]
    fn dependency_ready(self, fs_resolver: &FsResolver, ctx: &mut T) -> anyhow::Result<Self> {
        let state = self
            .state
            .dependency_ready(fs_resolver, ctx)
            .with_context(|| format!("dependency_ready({:?})", self.src))?;
        tracing::debug!(?state, "ack");
        Ok(AssetNode { state, ..self })
    }

    #[instrument(target=TARGET_NAME, skip_all, fields(path=?self.src, ty=self.ty))]
    fn bytes_ready(
        self,
        others: &HashMap<Rc<Path>, AssetNode<T>>,
        data: Vec<u8>,
    ) -> anyhow::Result<(Vec<PathBuf>, bool, Self)> {
        let (deps, all_deps_ready, state) = self
            .state
            .bytes_ready(others, data)
            .with_context(|| format!("bytes_read({:?})", self.src))?;
        Ok((deps, all_deps_ready, AssetNode { state, ..self }))
    }
}

enum AssetNodeState<T> {
    PendingFsRequest { on_bytes_ready: OnBytesReady, on_deps_ready: OnDepsReady<T> },
    BytesReady { data: Vec<u8>, deps_not_loaded: usize, on_deps_ready: OnDepsReady<T> },
    Initialized,
}

impl<T> AssetNodeState<T> {
    fn dependency_ready(self, fs_resolver: &FsResolver, ctx: &mut T) -> anyhow::Result<Self> {
        let AssetNodeState::BytesReady { data, deps_not_loaded, on_deps_ready } = self else {
            tracing::warn!("not waiting for deps");
            return Ok(self);
        };

        let new_state = if deps_not_loaded > 1 {
            AssetNodeState::BytesReady { deps_not_loaded: deps_not_loaded - 1, data, on_deps_ready }
        } else {
            on_deps_ready(ctx, fs_resolver, data)?;
            AssetNodeState::Initialized
        };
        Ok(new_state)
    }

    fn bytes_ready(
        self,
        others: &HashMap<Rc<Path>, AssetNode<T>>,
        data: Vec<u8>,
    ) -> anyhow::Result<(Vec<PathBuf>, bool, Self)> {
        let AssetNodeState::PendingFsRequest { on_bytes_ready, on_deps_ready } = self else {
            tracing::warn!("not wating for bytes");
            return Ok((Vec::new(), false, self));
        };
        let deps = on_bytes_ready(&data)?;
        let deps_not_loaded = deps
            .iter()
            .filter(|dep| {
                others
                    .get(dep.as_path())
                    .map(|x| !x.state.is_initialized())
                    .unwrap_or(true)
            })
            .count();
        Ok((
            deps,
            deps_not_loaded == 0,
            AssetNodeState::BytesReady { data, deps_not_loaded, on_deps_ready },
        ))
    }

    fn is_initialized(&self) -> bool {
        matches!(self, AssetNodeState::Initialized)
    }
}

impl<T> fmt::Debug for AssetNodeState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingFsRequest { .. } => write!(f, "PendingFsRequest"),
            Self::BytesReady { deps_not_loaded, .. } => f
                .debug_struct("BytesReady")
                .field("deps_left", deps_not_loaded)
                .finish(),
            Self::Initialized => write!(f, "Initialized"),
        }
    }
}

type OnBytesReady = Box<dyn FnOnce(&[u8]) -> anyhow::Result<Vec<PathBuf>>>;
type OnDepsReady<T> = Box<dyn FnOnce(&mut T, &FsResolver, Vec<u8>) -> anyhow::Result<()>>;

#[cfg(test)]
mod tests {
    use super::AssetNodeState;
    use std::{path::Path, sync::LazyLock};
    use test_lib::*;

    static PATH_A: LazyLock<&Path> = LazyLock::new(|| Path::new("A"));
    static PATH_B: LazyLock<&Path> = LazyLock::new(|| Path::new("B"));

    #[test]
    fn simple_upload() {
        init_test_logger();
        let mut manager = make_manager();

        manager.create_asset_task(simple_node(*PATH_A, vec![]));
        manager.on_file_ready(&mut (), file_ok(*PATH_A)).unwrap();
        assert!(manager.nodes[*PATH_A].state.is_initialized());
    }

    #[test]
    fn simple_upload_err() {
        init_test_logger();
        let mut manager = make_manager();

        manager.create_asset_task(simple_node(*PATH_A, vec![]));
        manager
            .on_file_ready(&mut (), file_err(*PATH_A))
            .unwrap_err();
        assert!(!manager.nodes.contains_key(*PATH_A));
    }

    #[test]
    fn simple_upload_with_deps() {
        init_test_logger();
        let mut manager = make_manager();

        manager.create_asset_task(simple_node(*PATH_A, vec![PATH_B.to_path_buf()]));
        manager.on_file_ready(&mut (), file_ok(*PATH_A)).unwrap();
        assert!(manager.nodes.contains_key(*PATH_A));
        assert!(manager.dependents.contains_key(*PATH_B));
        assert!(matches!(
            manager.nodes[*PATH_A].state,
            AssetNodeState::BytesReady { .. }
        ));

        manager.create_asset_task(simple_node(*PATH_B, vec![]));
        manager.on_file_ready(&mut (), file_ok(*PATH_B)).unwrap();
        assert!(manager.nodes[*PATH_A].state.is_initialized());
    }

    mod test_lib {
        use std::cell::RefCell;
        use std::io;
        use std::path::{Path, PathBuf};
        use std::rc::Rc;

        use hashbrown::HashSet;
        use mimiq::{FileReady, FsServer};
        use tracing::Level;

        use crate::manager::{AssetManager, AssetNode};
        use crate::prefab::PrefabFactory;

        pub fn make_manager() -> AssetManager<()> {
            let server = Rc::new(TrackingFsServer { requests: RefCell::new(HashSet::new()) });
            AssetManager::new(server, PrefabFactory::new())
        }

        struct TrackingFsServer {
            requests: RefCell<HashSet<PathBuf>>,
        }

        impl FsServer for TrackingFsServer {
            fn load_file(&self, path: &Path) {
                let new = self.requests.borrow_mut().insert(path.to_path_buf());
                assert!(new, "duplicate request for {path:?}");
            }
        }

        pub fn simple_node(path: &Path, deps: Vec<PathBuf>) -> AssetNode<()> {
            AssetNode::new(
                path.into(),
                "test_node",
                Box::new(move |_| Ok(deps)),
                Box::new(|_, _, _| Ok(())),
            )
        }

        pub fn file_ok(path: &Path) -> FileReady {
            FileReady { path: path.to_path_buf(), bytes_result: Ok(Vec::new()) }
        }

        pub fn file_err(path: &Path) -> FileReady {
            FileReady {
                path: path.to_path_buf(),
                bytes_result: Err(io::Error::other("test error")),
            }
        }

        pub fn init_test_logger() {
            // Tests run in parallel, so some might have already created the logger.
            let _ = tracing_subscriber::fmt()
                .with_level(true)
                .with_max_level(Level::DEBUG)
                .try_init();
        }
    }
}
