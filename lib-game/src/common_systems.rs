use crate::components::*;
use crate::prelude::*;

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
    for (projectile_entity, (tf, attack_team, col_q, dmg)) in
        &mut world.query::<(&Transform, &Team, &col_query::Damage, &Damage)>()
    {
        let has_knockback = world.get::<&KnockbackTag>(projectile_entity).is_ok();
        for collide_with in collisions.collisions_for(col_q) {
            let Ok(mut query) = world.query_one::<(&Team, &mut Hp, &Defence)>(*collide_with) else {
                continue;
            };
            let Some((collided_team, hp, def)) = query.get() else {
                continue;
            };
            if *collided_team == *attack_team {
                continue;
            }
            hp.damage(dmg_formula(dmg.heavy, def.heavy) + dmg_formula(dmg.light, def.light));
            if let Ok(mut knock) = world.get::<&mut KnockbackState>(*collide_with)
                && has_knockback
            {
                knock.knockback_direction = Vec2::from_angle(tf.angle);
                knock.knockback_left = knock.knockback_length;
            };
        }
    }
}

fn dmg_formula(dmg: i32, def: i32) -> i32 {
    ((dmg as f32) * (1.0 - def as f32 / 100.0)) as i32
}
