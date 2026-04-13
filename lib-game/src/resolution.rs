use crate::prelude::*;

pub const SCREEN_WIDTH: u32 = 480;
pub const SCREEN_HEIGHT: u32 = 270;

/// Returns a matrix for converting native coordinates into screen (world) coordinates.
pub fn native_to_screen(native_width: u32, native_height: u32) -> Mat3 {
    let (left, right, top, bottom) =
        crate::resolution::native_scaled_quad_points(native_width, native_height);

    let screen_to_dev = mat3_ortho(left, right, bottom, top);
    let world_to_dev = mat3_ortho(0.0, SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32, 0.0);
    world_to_dev.inverse() * screen_to_dev
}

/// Sometimes the screen may not fit nicely into the native screen fully.
/// This function returns quad points for rendering the screen centered.
/// Return value: (left, right, top, bottom)
pub fn native_scaled_quad_points(native_width: u32, native_height: u32) -> (f32, f32, f32, f32) {
    let native_sz = vec2(native_width as f32, native_height as f32);
    let native_screen_sz = native_scaled_dimensions(native_width, native_height);

    // Round to not create halfpixels, as it will ruin the pixelart
    let sz_remaining_half = ((native_screen_sz - native_sz) * 0.5).round();
    let left_top = vec2(0.0, 0.0) - sz_remaining_half;
    let right_bottom = native_screen_sz - sz_remaining_half;

    (left_top.x, right_bottom.x, left_top.y, right_bottom.y)
}

/// Computes the size of the screen in the native resolution, that can be used to render it.
pub fn native_scaled_dimensions(native_width: u32, native_height: u32) -> Vec2 {
    let screen_sz = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    screen_sz * native_scaling_factor(native_width, native_height)
}

/// Computes the factor of scaling the pixels
pub fn native_scaling_factor(native_width: u32, native_height: u32) -> f32 {
    let native_sz = vec2(native_width as f32, native_height as f32);
    let screen_sz = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    (native_sz / screen_sz).ceil().max_element()
}

fn mat3_ortho(left: f32, right: f32, bottom: f32, top: f32) -> Mat3 {
    let a = 2.0 / (right - left);
    let b = 2.0 / (top - bottom);
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);

    Mat3::from_cols(
        Vec3::new(a, 0.0, 0.0),
        Vec3::new(0.0, b, 0.0),
        Vec3::new(tx, ty, 1.0),
    )
}
