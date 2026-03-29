use crate::components::*;
use crate::prelude::*;

pub fn step(dt: f32, world: &mut World) {
    let query = world.query_mut::<(&ProjectileTag, &Transform, &mut KinematicControl)>();
    for (_, (tag, tf, kin)) in query {
        kin.dr = (dt * tag.speed) * Vec2::from_angle(tf.angle);
    }
}

pub fn impact(world: &mut World, collisions: &CollisionSolver, cmds: &mut CommandBuffer) {
    let mut query = world
        .query::<(&KinematicControl, &Team, &col_query::Damage)>()
        .with::<&ProjectileTag>();
    for (projectile_entity, (kin, projectile_team, col_q)) in &mut query {
        if kin.collided {
            cmds.despawn(projectile_entity);
            continue;
        }

        let mut collided = false;
        for collide_with in collisions.collisions_for(col_q) {
            let Ok(mut query) = world.query_one::<(&Team, &mut Hp)>(*collide_with) else {
                continue;
            };
            let Some((collided_team, hp)) = query.get() else {
                continue;
            };
            if *collided_team == *projectile_team {
                continue;
            }
            hp.hp -= 1;
            collided = true;
        }

        if collided {
            cmds.despawn(projectile_entity);
        }
    }
}
