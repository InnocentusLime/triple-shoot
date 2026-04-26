use std::path::PathBuf;

use crate::prelude::*;

pub struct Win;

impl Win {
    pub fn make_state_request() -> StateRequest {
        let dependencies = ["atlas/win.png"];

        StateRequest {
            name: "win",
            constructor: Box::new(Self::new_state),
            dependencies: dependencies.into_iter().map(PathBuf::from).collect(),
        }
    }

    pub fn new_state(resources: &mut Resources, cmds: &mut CommandBuffer) -> Box<dyn State> {
        let win = resources.textures.resolve("atlas/win.png").unwrap();
        cmds.spawn((
            Transform::from_xy(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0),
            Sprite {
                layer: 0,
                texture: win,
                tex_rect_pos: uvec2(0, 0),
                tex_rect_size: uvec2(256, 256),
                color: Color::from_hex(0xffffffff),
                sort_offset: 0.0,
                local_offset: Vec2::ZERO,
            },
        ));
        Box::new(Win)
    }
}

impl State for Win {
    fn handle_command(&mut self, _resources: &mut Resources, _cmd: &DebugCommand) -> bool {
        false
    }

    fn input(
        &mut self,
        _dt: f32,
        _input_model: &InputModel,
        _resources: &mut Resources,
        _cmds: &mut CommandBuffer,
    ) {
    }

    fn plan_collision_queries(
        &mut self,
        _dt: f32,
        _resources: &mut Resources,
        _cmds: &mut CommandBuffer,
    ) {
    }

    fn update(
        &mut self,
        _dt: f32,
        _resources: &mut Resources,
        _collisions: &CollisionSolver,
        _cmds: &mut CommandBuffer,
    ) -> Option<StateRequest> {
        None
    }

    fn ui(&mut self, _resources: &mut Resources, _out: &mut Vec<UiElement>) {}
}
