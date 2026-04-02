use crate::prelude::*;

pub struct MainGame {
    do_ai: bool,
    do_player_controls: bool,
}

impl MainGame {
    pub fn make_state_request() -> StateRequest {
        StateRequest {
            name: "main game",
            constructor: Box::new(Self::new_state),
            dependencies: vec!["prefab/player.json".into()],
        }
    }

    pub fn new_state(_resources: &mut Resources) -> Box<dyn State> {
        Box::new(MainGame { do_player_controls: true, do_ai: true })
    }
}

impl State for MainGame {
    fn handle_command(&mut self, _resources: &mut Resources, cmd: &DebugCommand) -> bool {
        match &*cmd.command {
            "nopl" => self.do_player_controls = false,
            "pl" => self.do_player_controls = true,
            "noai" => self.do_ai = false,
            "ai" => self.do_ai = true,
            _ => return false,
        }
        true
    }

    fn plan_collision_queries(
        &mut self,
        dt: f32,
        resources: &mut Resources,
        cmds: &mut CommandBuffer,
    ) {
        let spawn_poses = [
            vec2(0.0, 0.0),
            vec2(512.0, 0.0),
            vec2(512.0, 512.0),
            vec2(0.0, 512.0),
            vec2(512.0, 256.0),
        ];

        for (_, spawn) in &mut resources.world.query::<&mut EnemySpawner>() {
            spawn.next_spawn -= dt;
            if spawn.next_spawn > 0.0 {
                continue;
            }
            spawn.next_spawn = spawn.spawn_time;
            let pos = fastrand::choice(spawn_poses).unwrap();
            spawn_prefab(
                cmds,
                &resources,
                spawn.enemy_prefab,
                Transform::from_pos(pos),
            );
        }
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

    fn input(
        &mut self,
        dt: f32,
        input_model: &InputModel,
        resources: &mut Resources,
        cmds: &mut CommandBuffer,
    ) {
        let mut player_pos = Vec2::ZERO;
        let mut query = resources
            .world
            .query::<(&mut Transform, &mut KinematicControl, &PlayerArsenal)>();
        for (_, (tf, kin, arsenal)) in &mut query {
            kin.dr = 32.0 * dt * input_model.player_move_direction;
            let pos = tf.pos + 32.0 * input_model.player_aim_direction;
            player_pos = tf.pos;

            if input_model.shoot_pressed {
                info!("shoot");
                spawn_prefab(
                    cmds,
                    resources,
                    arsenal.bullet_prefab,
                    Transform { pos, angle: input_model.player_aim_direction.to_angle() },
                );
            }
        }
        std::mem::drop(query);

        let mut query = resources
            .world
            .query::<(&Transform, &mut KinematicControl, &NpcAi)>();
        for (this, (tf, kin, ai)) in &mut query {
            match ai {
                NpcAi::JustFollowPlayer => {
                    let walk_dir = (player_pos - tf.pos).normalize_or_zero();
                    let steer_dir = steer_dir(&resources.world, this, tf.pos);
                    let move_dir = (0.4 * walk_dir + 0.8 * steer_dir).normalize_or_zero();

                    kin.dr = (24.0 * dt) * move_dir;
                }
            }
        }
    }
}

fn steer_dir(world: &World, this: Entity, pos: Vec2) -> Vec2 {
    const SEPARATION_RADIUS: f32 = 20.0;

    let mut result = Vec2::ZERO;
    for (other, (tf, team)) in &mut world.query::<(&Transform, &Team)>() {
        if *team != Team::Enemy {
            continue;
        }
        if other == this {
            continue;
        }

        let dr = pos - tf.pos;
        let dist = dr.length();
        if dist < SEPARATION_RADIUS {
            result += dr.normalize_or_zero();
        }
    }

    result.normalize_or_zero()
}
