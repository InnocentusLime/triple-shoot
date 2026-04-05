use crate::components::*;
use crate::prelude::*;

pub fn step(dt: f32, world: &mut World) {
    let query = world.query_mut::<(&ProjectileTag, &Transform, &mut KinematicControl)>();
    for (_, (tag, tf, kin)) in query {
        kin.dr = (dt * tag.speed) * Vec2::from_angle(tf.angle);
    }
}

pub fn impact(world: &mut World, collisions: &CollisionSolver, cmds: &mut CommandBuffer) {
    let mut query = world.query::<(
        &KinematicControl,
        &Team,
        &col_query::Damage,
        &mut ProjectileTag,
    )>();
    for (projectile_entity, (kin, projectile_team, col_q, tag)) in &mut query {
        if kin.collided {
            cmds.despawn(projectile_entity);
            continue;
        }

        let mut collided = false;
        for collide_with in collisions.collisions_for(col_q) {
            let Ok(mut query) = world.query_one::<(&Team, &Hp)>(*collide_with) else {
                continue;
            };
            let Some((collided_team, collided_hp)) = query.get() else {
                continue;
            };
            if *collided_team == *projectile_team || collided_hp.cooling_down() {
                continue;
            }
            collided = true;
        }

        if collided && tag.pierce_count == 0 {
            cmds.despawn(projectile_entity);
        } else {
            tag.pierce_count = tag.pierce_count.saturating_sub(1)
        }
    }
}
