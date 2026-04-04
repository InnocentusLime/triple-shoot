use crate::components::*;
use crate::prelude::*;

pub fn tick(dt: f32, world: &mut World, cmds: &mut CommandBuffer) {
    for (entity, lifetime) in world.query_mut::<&mut Lifetime>() {
        if lifetime.time_left > 0.0 {
            lifetime.time_left -= dt;
        } else {
            cmds.despawn(entity);
        }
    }
}
