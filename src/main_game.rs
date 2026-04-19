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
            dependencies: vec![
                "prefab/player.json".into(),
                "prefab/wall_horiz.json".into(),
                "prefab/wall_vert.json".into(),
            ],
        }
    }

    pub fn new_state(resources: &mut Resources, cmds: &mut CommandBuffer) -> Box<dyn State> {
        let wall_horiz = resources.prefabs.resolve("prefab/wall_horiz.json").unwrap();
        let wall_vert = resources.prefabs.resolve("prefab/wall_vert.json").unwrap();
        let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
        let play_off = vec2(resources.game_field_width, resources.game_field_height) * 0.5;

        spawn_prefab(
            cmds,
            resources,
            wall_horiz,
            Transform::from_pos(center - play_off * Vec2::Y),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_horiz,
            Transform::from_pos(center + play_off * Vec2::Y),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_vert,
            Transform::from_pos(center - play_off * Vec2::X),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_vert,
            Transform::from_pos(center + play_off * Vec2::X),
        );
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
        _dt: f32,
        _resources: &mut Resources,
        _cmds: &mut CommandBuffer,
    ) {
    }

    fn update(
        &mut self,
        _dt: f32,
        resources: &mut Resources,
        _collisions: &CollisionSolver,
        cmds: &mut CommandBuffer,
    ) -> Option<StateRequest> {
        for (ent, (col_q, ammo)) in &mut resources
            .world
            .query::<(&col_query::Interaction, &AmmoPickup)>()
        {
            if !col_q.has_collided() {
                continue;
            }
            cmds.despawn(ent);
            for (_, data) in &mut resources.world.query::<&mut PlayerData>() {
                let gun = data.get_gun(ammo.weapon);
                let new_ammo = gun.max_ammo.min(gun.ammo + ammo.value);
                data.set_gun(ammo.weapon, GunEntry { ammo: new_ammo, ..gun });
            }
        }

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
            .query::<(&mut Transform, &mut KinematicControl, &mut PlayerData)>();
        for (_, (tf, kin, data)) in &mut query {
            kin.dr = data.speed * dt * input_model.player_move_direction;
            let spawn_pos = tf.pos + 8.0 * input_model.player_aim_direction;
            player_pos = tf.pos;

            dump!("Player weapn: {:?}", data.current_weapon);
            dump!("Shotgun: {} / {}", data.shotgun.ammo, data.shotgun.max_ammo);
            dump!("Rifle: {} / {}", data.rifle.ammo, data.rifle.max_ammo);
            dump!(
                "Cooldown: {:.2}. Can shoot: {}",
                data.next_shoot,
                data.next_shoot <= 0.0
            );

            if let Some(weapon_request) = input_model.player_weapon_request
                && data.next_shoot <= 0.0
            {
                data.current_weapon = weapon_request
            }

            if input_model.shoot_down && data.next_shoot <= 0.0 {
                shoot(
                    resources,
                    data,
                    cmds,
                    input_model.player_aim_direction,
                    spawn_pos,
                );
            }

            if data.next_shoot > 0.0 {
                data.next_shoot -= dt;
            }
        }
        std::mem::drop(query);

        let mut query = resources
            .world
            .query::<(&Transform, &mut KinematicControl, &NpcAi)>();
        for (this, (tf, kin, ai)) in &mut query {
            match ai {
                NpcAi::JustFollowPlayer { speed } => {
                    let walk_dir = (player_pos - tf.pos).normalize_or_zero();
                    let steer_dir = steer_dir(&resources.world, this, tf.pos);
                    let move_dir = (0.4 * walk_dir + 0.8 * steer_dir).normalize_or_zero();

                    kin.dr = (*speed * dt) * move_dir;
                }
            }
        }
    }
}

fn shoot(
    resources: &Resources,
    data: &mut PlayerData,
    cmds: &mut CommandBuffer,
    aim_dir: Vec2,
    spawn_pos: Vec2,
) {
    let gun = data.get_gun(data.current_weapon);
    if gun.ammo == 0 {
        return;
    }

    let spread_angle = gun.spread_angle.to_radians();
    let base = -spread_angle / 2.0;
    let delta = spread_angle / (gun.bullets_in_spread as f32);
    for offset_idx in 0..gun.bullets_in_spread {
        let offset = base + (offset_idx as f32) * delta;
        spawn_prefab(
            cmds,
            resources,
            gun.bullet_prefab,
            Transform { pos: spawn_pos, angle: aim_dir.to_angle() + offset },
        );
    }

    data.next_shoot = gun.shoot_cooldown;
    data.set_gun(data.current_weapon, GunEntry { ammo: gun.ammo - 1, ..gun });
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
