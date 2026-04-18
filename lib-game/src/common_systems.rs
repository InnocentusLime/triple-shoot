use crate::components::*;
use crate::prelude::*;
use crate::spawn_prefab;

pub fn tick_knockback(dt: f32, world: &mut World) {
    for (_, (knock, kinematic)) in world.query_mut::<(&mut KnockbackState, &mut KinematicControl)>()
    {
        if knock.knockback_left > 0.0 {
            let t = 1.0 - knock.knockback_left / knock.knockback_length;
            let k = 3.0 * (2.0f32).powf(-7.0 * t);
            let dist = dt * k * knock.knockback_speed;
            kinematic.dr = dist * knock.knockback_direction;
            knock.knockback_left -= dt;
        }
    }
}

pub fn tick_lifetime(dt: f32, world: &mut World, cmds: &mut CommandBuffer) {
    for (entity, lifetime) in world.query_mut::<&mut Lifetime>() {
        if lifetime.time_left > 0.0 {
            lifetime.time_left -= dt;
        } else {
            cmds.despawn(entity);
        }
    }
}

pub fn despawn_on_low_hp(world: &mut World, cmds: &mut CommandBuffer) {
    for (entity, hp) in world.query_mut::<&Hp>() {
        if hp.hp <= 0 {
            cmds.despawn(entity);
        }
    }
}

pub fn tick_hp(dt: f32, world: &mut World) {
    for (_, hp) in world.query_mut::<&mut Hp>() {
        if hp.cooldown > 0.0 {
            hp.cooldown -= dt;
        }
    }
}

pub fn do_damage(world: &mut World, collisions: &CollisionSolver) {
    let mut view = world.view::<(&Team, &mut Hp, &Defence)>();
    for (_, (attack_team, col_q, dmg)) in &mut world.query::<(&Team, &col_query::Damage, &Damage)>()
    {
        for collide_with in collisions.collisions_for(col_q) {
            let Some((collided_team, hp, def)) = view.get_mut(*collide_with) else {
                continue;
            };
            if *collided_team == *attack_team {
                continue;
            }
            hp.damage(dmg_formula(dmg.heavy, def.heavy) + dmg_formula(dmg.light, def.light));
        }
    }
}

fn dmg_formula(dmg: i32, def: i32) -> i32 {
    ((dmg as f32) * (1.0 - def as f32 / 100.0)) as i32
}

// TODO: consider scaling knockback according to the defence
pub fn do_knockback(world: &mut World, collisions: &CollisionSolver) {
    let mut view = world.view::<(&Team, &mut KnockbackState, &Hp)>();
    for (_, (tf, attack_team, col_q, _)) in
        &mut world.query::<(&Transform, &Team, &col_query::Damage, &KnockbackTag)>()
    {
        for collide_with in collisions.collisions_for(col_q) {
            let Some((collided_team, knock, hp)) = view.get_mut(*collide_with) else {
                continue;
            };
            if !hp.cooling_down() {
                continue;
            }
            if *collided_team == *attack_team {
                continue;
            }
            knock.knockback_direction = Vec2::from_angle(tf.angle);
            knock.knockback_left = knock.knockback_length;
        }
    }
}

pub fn tick_mob_spawners(dt: f32, resources: &mut Resources) {
    for (_, spawner) in resources.world.query_mut::<&mut MobSpawner>() {
        #[cfg(feature = "dbg")]
        let src = resources.prefabs.inverse_resolve(spawner.prefab);
        dump!(
            "spawner ({src:?}): cooldown={:.2} quota={}",
            spawner.cooldown,
            spawner.quota
        );
        if spawner.cooldown > 0.0 {
            spawner.cooldown -= dt;
        }
    }
}

pub fn tick_global_spawn(dt: f32, resources: &mut Resources, cmds: &mut CommandBuffer) {
    for (_, spawn) in &mut resources.world.query::<&mut SpawnDirector>() {
        spawn.next_spawn -= dt;
        if spawn.next_spawn > 0.0 {
            continue;
        }
        spawn.next_spawn = spawn.spawn_time;
        let spawn_pos =
            make_random_spawn_pos(resources.game_field_width, resources.game_field_height);
        let prefab = pick_random_spawner(&resources.world);
        if prefab != INVALID_ASSET {
            spawn_prefab(cmds, resources, prefab, Transform::from_pos(spawn_pos));
        }
    }
}

fn make_random_spawn_pos(game_field_width: f32, game_field_height: f32) -> Vec2 {
    const BUMP: f32 = 32.0;

    let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
    let off = vec2(game_field_width, game_field_height) / 2.0 - Vec2::splat(BUMP);
    let noise_increment = fastrand::i32(-3..3);
    let spawnpoints = [
        center - Vec2::X * off + Vec2::Y * (32.0 * noise_increment as f32),
        center + Vec2::X * off + Vec2::Y * (32.0 * noise_increment as f32),
        center - Vec2::Y * off + Vec2::X * (32.0 * noise_increment as f32),
        center + Vec2::Y * off + Vec2::X * (32.0 * noise_increment as f32),
    ];
    fastrand::choice(spawnpoints).unwrap()
}

fn pick_random_spawner(world: &World) -> AssetKey {
    let mut total_weight = 0;
    for (_, spawn) in &mut world.query::<&MobSpawner>() {
        if !spawn.can_spawn() {
            continue;
        }
        total_weight += spawn.weight;
    }

    if total_weight == 0 {
        return INVALID_ASSET;
    }

    let mut chosen_weight = fastrand::u32(0..total_weight);
    for (_, spawn) in &mut world.query::<&mut MobSpawner>() {
        if !spawn.can_spawn() {
            continue;
        }
        if chosen_weight < spawn.weight {
            spawn.quota = spawn.quota.saturating_sub(1);
            spawn.cooldown = spawn.cooldown_length + fastrand::f32();
            return spawn.prefab;
        }
        chosen_weight -= spawn.weight;
    }
    return INVALID_ASSET;
}
