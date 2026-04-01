use crate::components::*;
use crate::prelude::*;

pub fn despawn_on_low_hp(world: &mut World, cmds: &mut CommandBuffer) {
    for (entity, hp) in world.query_mut::<&Hp>() {
        if hp.hp <= 0 {
            cmds.despawn(entity);
        }
    }
}

pub fn tick(dt: f32, world: &mut World) {
    for (_, hp) in world.query_mut::<&mut Hp>() {
        if hp.cooldown > 0.0 {
            hp.cooldown -= dt;
        }
    }
}
