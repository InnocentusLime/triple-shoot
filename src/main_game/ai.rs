use crate::prelude::*;

const BOIDS_SEPARATION_RADIUS: f32 = 20.0;

pub fn think(dt: f32, resources: &Resources) {
    let Some(player_pos) = get_player_pos(&resources.world) else {
        return;
    };
    let mut query = resources
        .world
        .query::<(&Transform, &mut KinematicControl, &NpcAi)>();
    for (_, (tf, kin, ai)) in &mut query {
        match ai {
            NpcAi::JustFollowPlayer { speed } => {
                let move_dir = (player_pos - tf.pos).normalize_or_zero();
                kin.dr = (*speed * dt) * move_dir;
            }
        }
    }
}

pub fn boid_steering(resources: &Resources) {
    for (this, (this_tf, this_boid, this_kin)) in
        &mut resources
            .world
            .query::<(&Transform, &Boid, &mut KinematicControl)>()
    {
        let mut steer_dir = Vec2::ZERO;
        if this_boid.group == 0 {
            continue;
        }

        for (other, (other_tf, other_boid)) in &mut resources.world.query::<(&Transform, &Boid)>() {
            if other == this || this_boid.group & other_boid.group == 0 {
                continue;
            }

            let dr = this_tf.pos - other_tf.pos;
            let dist = dr.length();
            if dist < BOIDS_SEPARATION_RADIUS {
                steer_dir += dr.normalize_or_zero();
                steer_dir = steer_dir.normalize_or_zero();
            }
        }

        let (move_dir, len) = this_kin.dr.normalize_and_length();
        this_kin.dr = (0.4 * move_dir + 0.8 * steer_dir).normalize_or_zero() * len;
    }
}

fn get_player_pos(world: &World) -> Option<Vec2> {
    world
        .query::<(&Transform, &PlayerTag)>()
        .into_iter()
        .map(|(_, (x, _))| x.pos)
        .next()
}
