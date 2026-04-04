use crate::collisions::*;
use crate::components::*;

use hecs::World;
use mimiq::Color;
use mimiq::util::ShapeBatcher;

pub fn draw_physics_debug(world: &mut World, gizmos: &mut ShapeBatcher) {
    draw_bodies(world, gizmos);
    draw_queries::<0>(world, gizmos);
    draw_queries::<1>(world, gizmos);
    draw_queries::<2>(world, gizmos);
    draw_queries::<3>(world, gizmos);
    draw_queries::<4>(world, gizmos);
    draw_queries::<5>(world, gizmos);
    draw_queries::<6>(world, gizmos);
    draw_queries::<7>(world, gizmos);
}

fn draw_queries<const ID: usize>(world: &World, gizmos: &mut ShapeBatcher) {
    for (_, (tf, shape, query)) in &mut world.query::<(&Transform, &Shape, &CollisionQuery<ID>)>() {
        let color = if query.has_collided() {
            mimiq::Color::new(0.00, 0.93, 0.80, 1.00)
        } else {
            mimiq::GREEN
        };

        draw_shape_lines(gizmos, tf, shape, color);
    }
}

fn draw_bodies(world: &mut World, gizmos: &mut ShapeBatcher) {
    for (_, (tf, shape, _tag)) in world.query_mut::<(&Transform, &Shape, &BodyTag)>() {
        draw_shape(gizmos, tf, shape, mimiq::DARKBLUE);
    }
}

pub fn draw_shape(gizmos: &mut ShapeBatcher, tf: &Transform, shape: &Shape, color: Color) {
    match *shape {
        Shape::Rect { width, height } => gizmos.rect(color, tf.pos, vec2(width, height), tf.angle),
        Shape::Circle { radius } => gizmos.circle(color, tf.pos, radius),
    }
}

pub fn draw_shape_lines(gizmos: &mut ShapeBatcher, tf: &Transform, shape: &Shape, color: Color) {
    match *shape {
        Shape::Rect { width, height } => {
            gizmos.rect_lines(color, 1.0, tf.pos, vec2(width, height), tf.angle)
        }
        Shape::Circle { radius } => gizmos.circle_lines(color, 1.0, tf.pos, radius),
    }
}
