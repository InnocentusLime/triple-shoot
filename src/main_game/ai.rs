use crate::prelude::*;

const BOIDS_SEPARATION_RADIUS: f32 = 20.0;
const FOLLOWER_SPEEDUP_RADIUS: f32 = 128.0;

pub fn think(dt: f32, resources: &Resources) {
    let Some(player_pos) = get_player_pos(&resources.world) else {
        return;
    };
    let mut query = resources
        .world
        .query::<(&Transform, &mut KinematicControl, &mut NpcAi, &mut Boid)>();
    for (_, (tf, kin, ai, boid)) in &mut query {
        let dr_to_player = player_pos - tf.pos;
        let dir_to_player = dr_to_player.normalize_or_zero();
        match ai {
            NpcAi::JustFollowPlayer { speed } => {
                let movement_speed = if dr_to_player.length() >= FOLLOWER_SPEEDUP_RADIUS {
                    2.3 * *speed
                } else {
                    *speed
                };
                kin.dr = (movement_speed * dt) * dir_to_player;
            }
            NpcAi::Pouncer { state, speed, pounce_speed, wander_time, wind_time } => match state {
                PouncerState::Wandering { timer } => {
                    if *timer <= 0.0 {
                        *state = PouncerState::WindingUp { timer: *wind_time }
                    } else {
                        boid.group = 1;
                        *timer -= dt;
                        kin.dr = (*speed * dt) * dir_to_player;
                    }
                }
                PouncerState::WindingUp { timer } => {
                    if *timer <= 0.0 {
                        *state = PouncerState::Pouncing { dir: dir_to_player };
                    } else {
                        *timer -= dt;
                    }
                }
                PouncerState::Pouncing { dir } => {
                    if kin.collided {
                        let timer = *wander_time + fastrand::f32() * 1.4;
                        *state = PouncerState::Wandering { timer }
                    } else {
                        boid.group = 0;
                        kin.dr = (*pounce_speed * dt) * *dir;
                    }
                }
            },
        }
    }
}

pub fn sync_gfx(resources: &Resources) {
    for (_, (sprite, ai)) in &mut resources.world.query::<(&mut Sprite, &NpcAi)>() {
        match ai {
            NpcAi::JustFollowPlayer { .. } => (),
            NpcAi::Pouncer { state, .. } => match state {
                PouncerState::Wandering { .. } => {
                    sprite.tex_rect_pos = uvec2(633, 21);
                    sprite.tex_rect_size = uvec2(44, 37);
                }
                PouncerState::WindingUp { .. } | PouncerState::Pouncing { .. } => {
                    sprite.tex_rect_pos = uvec2(684, 29);
                    sprite.tex_rect_size = uvec2(45, 29);
                }
            },
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
