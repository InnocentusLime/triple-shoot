mod cmd;
mod screendump;

use std::path::Path;
use std::path::PathBuf;

use crate::components::*;
use crate::prelude::*;
use crate::{App, DebugCommand};

pub use cmd::*;
use mimiq::egui::TextBuffer;
pub use screendump::*;

use mimiq::egui::Context;

pub(crate) struct DebugStuff {
    pub cmd_center: CommandCenter,
    pub force_freeze: bool,
}

impl DebugStuff {
    pub(crate) fn new() -> Self {
        Self { cmd_center: CommandCenter::new(), force_freeze: false }
    }

    pub fn game_freeze_active(&self) -> bool {
        self.cmd_center.should_pause() || self.force_freeze
    }

    pub fn new_update(&mut self) {
        GLOBAL_DUMP.reset();
    }
}

impl App {
    pub fn dump_common_info(&mut self) {
        const TARGET_ASSET_NODES: &str = "lib_game::asset_nodes";
        const TARGET_ASSET_DEPENDENTS: &str = "lib_game::asset_dependents";

        let ent_count = self.resources.world.iter().count();
        let player_count = self
            .resources
            .world
            .query_mut::<&PlayerTag>()
            .into_iter()
            .count();

        for node in self.asset_manager.iter_node_debug() {
            dump!(target:TARGET_ASSET_NODES, "NODE: {:?} ({}) STATE: {} ({})", node.path, node.ty, node.state, node.deps_not_loaded);
        }

        for (asset, dependents) in self.asset_manager.iter_node_dependents() {
            dump!(target:TARGET_ASSET_DEPENDENTS, "{asset:?}: {dependents:?}");
        }

        // dump!("FPS: {:?}", get_fps());
        dump!("Entities: {ent_count}");
        dump!("Players: {player_count}");
        self.dump_archetypes();
        GLOBAL_DUMP.lock();
    }

    fn dump_archetypes(&self) {
        let mut total_archetypes = 0;
        for _arch in self.resources.world.archetypes() {
            total_archetypes += 1;
        }

        dump!("Total archetypes: {total_archetypes}");
    }

    pub fn debug_ui(&mut self, egui_ctx: &Context) {
        if let Some(cmd) = self.debug.cmd_center.show(egui_ctx) {
            if let Err(err) = self.handle_command(&cmd) {
                error!("fail: {err:#}");
            }
        }
        GLOBAL_DUMP.ui(egui_ctx);
    }

    fn handle_command(&mut self, cmd: &DebugCommand) -> anyhow::Result<()> {
        match cmd.command.as_str() {
            "f" => self.debug.force_freeze = true,
            "uf" => self.debug.force_freeze = false,
            "hw" => self.render.render_world = false,
            "sw" => self.render.render_world = true,
            "clw" => self.resources.world.clear(),
            "lass" => {
                if cmd.args.is_empty() {
                    anyhow::bail!("Not enough args");
                }

                self.queue_assets(std::iter::once(cmd.args[0].as_str()));
            }
            "dde" => {
                if cmd.args.is_empty() {
                    anyhow::bail!("Not enough args");
                }

                let dd_name = cmd.args[0].as_str();
                if !self.render.debug_draws.contains_key(dd_name) {
                    anyhow::bail!("No such debug draw: {:?}", dd_name);
                }
                self.render.enabled_debug_draws.insert(dd_name.to_owned());
                info!("Enabled debug draw {dd_name:?}");
            }
            "ddd" => {
                if cmd.args.is_empty() {
                    anyhow::bail!("Not enough args");
                }

                let dd_name = cmd.args[0].as_str();
                if !self.render.enabled_debug_draws.contains(dd_name) {
                    anyhow::bail!("No enabled debug draw: {:?}", dd_name);
                }
                self.render.enabled_debug_draws.remove(dd_name);
                info!("Disabled debug draw {dd_name:?}");
            }
            "spawn" => {
                if cmd.args.len() < 3 {
                    anyhow::bail!("Not enough args");
                }

                let Ok(x) = cmd.args[1].trim().parse::<f32>() else {
                    anyhow::bail!("Second argument is not a number");
                };
                let Ok(y) = cmd.args[2].trim().parse::<f32>() else {
                    anyhow::bail!("Third argument is not a number");
                };
                let prefab_path = make_prefab_path(cmd.args[0].as_str());
                let Some(prefab_handle) = self.resources.prefabs.resolve(&prefab_path) else {
                    anyhow::bail!("No such prefab: {prefab_path:?}");
                };
                let prefab = self.resources.prefabs.get(prefab_handle).unwrap();
                let entity = self.resources.world.spawn(prefab);
                if prefab.has::<Transform>() {
                    info!("prefab has Transform. Override to ({x:.2}, {y:.2})");
                    self.resources
                        .world
                        .insert_one(entity, Transform::from_xy(x, y))
                        .unwrap();
                }
                info!("Spawned {prefab_path:?} as {entity:?}");
            }
            #[cfg(feature = "dev-env")]
            "script" => {
                use anyhow::Context;

                if cmd.args.is_empty() {
                    anyhow::bail!("Not enough args");
                }

                let script_name = &cmd.args[0];
                let mut script_path = PathBuf::from_iter(["dbg_scripts", &script_name]);
                script_path.set_extension("txt");
                self.run_script(script_path)
                    .with_context(|| format!("running {script_name:?}"))?;
            }
            #[cfg(not(feature = "dev-env"))]
            "script" => {
                anyhow::bail!(
                    "\"script\" is not available. Did you build with \"dev-env\" feature?"
                );
            }
            unmatched => {
                if !self.state.handle_command(&mut self.resources, &cmd) {
                    anyhow::bail!("Unknown command: {unmatched:?}");
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "dev-env")]
    fn run_script(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        use anyhow::Context;
        use std::{fs, io::BufRead};

        let file = fs::File::open(path)?;
        for line in std::io::BufReader::new(file).lines() {
            let line = line.context("read line")?;
            let Some(cmd) = parse_command(&line) else {
                continue;
            };
            info!("run: {cmd:?}");
            self.handle_command(&cmd)
                .with_context(|| format!("run {:?}", cmd.command))?;
        }

        Ok(())
    }
}

fn make_prefab_path(name: impl AsRef<Path>) -> PathBuf {
    let mut prefab_path = PathBuf::from_iter([Path::new("prefab"), name.as_ref()]);
    prefab_path.set_extension("json");
    prefab_path
}
