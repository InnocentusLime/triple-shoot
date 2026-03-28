use std::path::{Path, PathBuf};

use lib_game::{AssetKey, Resources};

use serde::Deserialize;

pub fn register_components(prefab_factory: &mut lib_game::PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor_ctx(
        "player_arsenal",
        PlayerArsenalManifest::into_tag,
        PlayerArsenalManifest::dependencies,
    );
    prefab_factory.register_component::<BulletTag>("bullet");
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerArsenal {
    pub bullet_prefab: AssetKey,
}

#[derive(Debug, Deserialize)]
pub struct PlayerArsenalManifest {
    pub bullet_prefab: PathBuf,
}

impl PlayerArsenalManifest {
    pub fn into_tag(self, resources: &mut Resources) -> anyhow::Result<PlayerArsenal> {
        let Some(bullet_prefab) = resources.prefabs.resolve(&self.bullet_prefab) else {
            anyhow::bail!("No such prefab: {:?}", self.bullet_prefab);
        };
        Ok(PlayerArsenal { bullet_prefab })
    }

    pub fn dependencies(data: &serde_json::value::RawValue) -> anyhow::Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        pub struct Deps<'a> {
            #[serde(borrow)]
            pub bullet_prefab: &'a Path,
        }
        let deps = serde_json::from_str::<Deps>(data.get())?;
        Ok(vec![deps.bullet_prefab.into()])
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BulletTag;
