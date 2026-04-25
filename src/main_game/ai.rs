use crate::prelude::*;

pub fn think(dt: f32, resources: &Resources) {
    let Some(player_pos) = get_player_pos(&resources.world) else {
        return;
    };
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

fn get_player_pos(world: &World) -> Option<Vec2> {
    world
        .query::<(&Transform, &PlayerTag)>()
        .into_iter()
        .map(|(_, (x, _))| x.pos)
        .next()
}
