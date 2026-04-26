mod collisions;
mod common_systems;
mod components;
mod input;
mod prefab;
mod prelude;
mod projectile;
mod render;
mod resolution;
mod ui;

#[cfg(feature = "dbg")]
pub mod dbg;

pub mod state;

pub use collisions::CollisionSolver;
pub use components::*;
pub use input::{InputModel, WeaponId};
pub use prefab::spawn_prefab;
pub use prelude::*;
pub use state::*;
pub use ui::*;

use std::borrow::Cow;
use std::path::Path;
use std::rc::Rc;

#[macro_export]
#[cfg(feature = "dbg")]
macro_rules! dump {
    (target: $tgt:expr, $($arg:tt)+) => {
        $crate::dbg::GLOBAL_DUMP.put_line($tgt, std::format_args!($($arg)+));
    };
    ($($arg:tt)+) => {
        $crate::dbg::GLOBAL_DUMP.put_line(std::module_path!(), std::format_args!($($arg)+));
    };
}

#[macro_export]
#[cfg(not(feature = "dbg"))]
macro_rules! dump {
    (target:$tgt:expr, $($arg:tt)+) => {
        /* NOOP */
    };
    ($($arg:tt)+) => {
        /* NOOP */
    };
}

pub fn run(init: AppInit) {
    let conf = mimiq::Conf { fs_root: "assets".into(), ..Default::default() };
    mimiq::run::<_, App>(conf, init);
}

#[derive(Debug)]
pub struct DebugCommand<'a> {
    pub command: Cow<'a, str>,
    pub args: Vec<Cow<'a, str>>,
}

impl<'a> DebugCommand<'a> {
    pub fn to_owned(self) -> DebugCommand<'static> {
        DebugCommand {
            command: Cow::Owned(self.command.into_owned()),
            args: self
                .args
                .into_iter()
                .map(Cow::into_owned)
                .map(Cow::Owned)
                .collect(),
        }
    }
}

pub struct AppInit {
    pub initial_state: StateRequest,
    pub prefab_factory: PrefabFactory<Resources>,
}

/// The app run all the boilerplate code to make the game tick.
/// The following features are provided:
/// * State transitions and handling
/// * Debugging
/// * Physics handling
/// * Consistent tickrate timing
/// * Sound playing
/// * Integration with log-rs
/// * Drawing of the `dump!` macro
pub struct App {
    pub resources: Resources,
    pub render: render::Render,
    input: input::Input,
    col_solver: CollisionSolver,
    #[cfg(feature = "dbg")]
    debug: dbg::DebugStuff,
    cmds: CommandBuffer,
    asset_manager: AssetManager<Resources>,
    state: Box<dyn State>,
    queued_state: Option<StateRequest>,
}

impl mimiq::EventHandler<AppInit> for App {
    fn init(
        gl_ctx: Rc<mimiq::GlContext>,
        fs_server: Rc<dyn mimiq::FsServer>,
        init: AppInit,
    ) -> Self {
        let resources = Resources::new(gl_ctx);

        let mut prefab_factory = init.prefab_factory;
        prefab::register_libgame_components(&mut prefab_factory);
        let asset_manager = AssetManager::new(fs_server, prefab_factory);

        info!("Lib-game version {}. Started.", env!("CARGO_PKG_VERSION"));

        Self {
            render: render::Render::new(&resources),
            col_solver: CollisionSolver::new(),
            cmds: CommandBuffer::new(),
            input: input::Input::new(),
            asset_manager,
            #[cfg(feature = "dbg")]
            debug: dbg::DebugStuff::new(),
            resources,
            state: Box::new(BootState { redirect: Some(init.initial_state) }),
            queued_state: None,
        }
    }

    fn file_ready(&mut self, event: mimiq::FileReady) {
        if let Err(e) = self.asset_manager.on_file_ready(&mut self.resources, event) {
            error!("asset manager errored: {e:#}");
        }
        self.queue_assets(
            self.asset_manager
                .iter_assets_to_load()
                .cloned()
                .collect::<Vec<_>>(),
        );

        let Some(queued_state) = self.queued_state.take() else {
            return;
        };
        let is_state_ready = queued_state
            .dependencies
            .iter()
            .all(|dep| self.asset_manager.is_loaded(dep));
        if is_state_ready {
            info!("queued state ready: {:?}", queued_state.name);
            self.resources.world.clear();
            self.state = (queued_state.constructor)(&mut self.resources, &mut self.cmds);
            self.cmds.run_on(&mut self.resources.world);
        } else {
            self.queued_state = Some(queued_state)
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        #[cfg(not(feature = "dbg"))]
        let update = true;
        #[cfg(feature = "dbg")]
        let update = !self.debug.game_freeze_active();

        if !update {
            return;
        }

        let dt = dt.as_secs_f32();
        match (self.update_inner(dt), self.queued_state.is_some()) {
            (None, _) | (Some(_), true) => (),
            (Some(request), false) => {
                info!(
                    "new state ({:?}) requested with deps: {:?}",
                    request.name, request.dependencies
                );
                self.queue_assets(request.dependencies.iter());
                self.queued_state = Some(request);
            }
        }
    }

    fn window_event(&mut self, event: WindowEvent, _window: &Window) {
        self.input.handle_event(&event);

        if event == WindowEvent::RedrawRequested {
            self.render.ui_elements.clear();
            self.state
                .ui(&mut self.resources, &mut self.render.ui_elements);
            self.render.render(&mut self.resources);
        }
    }

    #[cfg(feature = "dbg")]
    fn egui(&mut self, egui_ctx: &mimiq::egui::Context) {
        self.dump_common_info();
        self.debug_ui(egui_ctx);
        self.debug.new_update();
    }
}

impl App {
    fn update_inner(&mut self, dt: f32) -> Option<StateRequest> {
        let input_model = self.input.get_input_model(&mut self.resources.world);
        self.state
            .input(dt, &input_model, &mut self.resources, &mut self.cmds);
        projectile::step(dt, &mut self.resources.world);
        common_systems::tick_knockback(dt, &mut self.resources.world);

        self.col_solver.import_colliders(&mut self.resources.world);
        self.col_solver
            .export_kinematic_moves(&mut self.resources.world);

        self.state
            .plan_collision_queries(dt, &mut self.resources, &mut self.cmds);
        self.cmds.run_on(&mut self.resources.world);

        self.col_solver
            .compute_collisions(&mut self.resources.world);
        common_systems::do_damage(&mut self.resources.world, &self.col_solver);
        common_systems::do_knockback(&mut self.resources.world, &self.col_solver);
        projectile::impact(&mut self.resources.world, &self.col_solver, &mut self.cmds);
        common_systems::despawn_on_low_hp(&mut self.resources.world, &mut self.cmds);

        let res = self
            .state
            .update(dt, &mut self.resources, &self.col_solver, &mut self.cmds);
        common_systems::tick_hp(dt, &mut self.resources.world);
        common_systems::tick_lifetime(dt, &mut self.resources.world, &mut self.cmds);
        self.cmds.run_on(&mut self.resources.world);

        self.resources.world.flush();
        self.input.update();
        res
    }

    fn queue_assets<P: AsRef<Path>>(&mut self, asset_list: impl IntoIterator<Item = P>) {
        for unloaded in asset_list.into_iter() {
            let unloaded = unloaded.as_ref();
            if unloaded.starts_with("atlas/") {
                self.asset_manager
                    .load_image(unloaded, Resources::init_texture);
                continue;
            }
            if unloaded.starts_with("prefab/") {
                self.asset_manager
                    .load_prefab(unloaded, Resources::init_prefab);
                continue;
            }
            warn!("unknown dep: {unloaded:?}");
        }
    }
}

pub struct Resources {
    pub world: World,
    pub gl_ctx: Rc<mimiq::GlContext>,
    pub sprite_pipeline: mimiq::Pipeline<mimiq::util::BasicSpritePipelineMeta>,
    pub basic_pipeline: mimiq::Pipeline<mimiq::util::BasicPipelineMeta>,
    pub gamescreen: mimiq::RenderPass,
    pub textures: AssetContainer<mimiq::Texture2D>,
    pub prefabs: AssetContainer<BuiltEntityClone>,
    pub game_field_width: f32,
    pub game_field_height: f32,
}

impl Resources {
    pub fn new(gl_ctx: Rc<mimiq::GlContext>) -> Self {
        let gamescreen_texture = gl_ctx.new_empty_texture(
            crate::resolution::SCREEN_WIDTH,
            crate::resolution::SCREEN_HEIGHT,
            mimiq::Texture2DParams {
                internal_format: mimiq::Texture2DFormat::RGBA8,
                min_filter: mimiq::FilterMode::Nearest,
                mag_filter: mimiq::FilterMode::Nearest,
                ..Default::default()
            },
        );

        Resources {
            world: World::new(),
            sprite_pipeline: gl_ctx.new_pipeline(),
            gamescreen: gl_ctx.new_render_pass(vec![gamescreen_texture], None),
            basic_pipeline: gl_ctx.new_pipeline(),
            textures: AssetContainer::new(),
            prefabs: AssetContainer::new(),
            gl_ctx,
            game_field_width: 400.0,
            game_field_height: 400.0,
        }
    }

    fn init_prefab(&mut self, _fs_resolver: &FsResolver, prefab: BuiltEntityClone, src: &Path) {
        self.prefabs.insert(src, prefab);
    }

    fn init_texture(
        &mut self,
        _fs_resolver: &FsResolver,
        image: mimiq::image::DynamicImage,
        src: &Path,
    ) {
        let tex = self.gl_ctx.new_texture(
            image,
            mimiq::Texture2DParams {
                internal_format: mimiq::Texture2DFormat::RGBA8,
                wrap: mimiq::TextureWrap::Clamp,
                min_filter: mimiq::FilterMode::Nearest,
                mag_filter: mimiq::FilterMode::Nearest,
            },
        );
        self.textures.insert(src, tex);
    }
}
