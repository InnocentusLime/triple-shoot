use crate::components::*;
use crate::prelude::*;

pub fn step(dt: f32, world: &mut World) {
    let query = world.query_mut::<(&ProjectileTag, &Transform, &mut KinematicControl)>();
    for (_, (tag, tf, kin)) in query {
        kin.dr = (dt * tag.speed) * Vec2::from_angle(tf.angle);
    }
}

pub fn impact(world: &mut World, _collisions: &CollisionSolver, cmds: &mut CommandBuffer) {
    let query = world
        .query_mut::<(&KinematicControl, &col_query::Damage)>()
        .with::<&ProjectileTag>();
    for (entity, (kin, col_q)) in query {
        if kin.collided {
            cmds.despawn(entity);
            continue;
        }

        if col_q.has_collided() {
            cmds.despawn(entity);
            continue;
        }
    }
}
