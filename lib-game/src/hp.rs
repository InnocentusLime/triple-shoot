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

pub fn get_damaged(world: &mut World, collisions: &CollisionSolver) {
    for (_, (attack_team, col_q)) in &mut world.query::<(&Team, &col_query::Damage)>() {
        for collide_with in collisions.collisions_for(col_q) {
            let Ok(mut query) = world.query_one::<(&Team, &mut Hp)>(*collide_with) else {
                continue;
            };
            let Some((collided_team, hp)) = query.get() else {
                continue;
            };
            if *collided_team == *attack_team {
                continue;
            }
            hp.damage(1);
        }
    }
}
