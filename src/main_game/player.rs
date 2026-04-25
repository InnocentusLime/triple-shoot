use crate::prelude::*;

pub fn input(dt: f32, input_model: &InputModel, resources: &Resources, cmds: &mut CommandBuffer) {
    let mut query = resources
        .world
        .query::<(&mut Transform, &mut KinematicControl, &mut PlayerData)>();
    for (_, (tf, kin, data)) in &mut query {
        kin.dr = data.speed * dt * input_model.player_move_direction;
        let spawn_pos = tf.pos + 8.0 * input_model.player_aim_direction;

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
