use crate::prelude::*;

use mimiq::util::InputTracker;

#[derive(Debug)]
pub struct InputModel {
    pub player_move_direction: Vec2,
    pub shoot_pressed: bool,
}

pub struct Input {
    /// tracks the mouse cursor position in world coordinates.
    cursor_pos: Vec2,
    buttons: InputTracker,
}

impl Input {
    pub fn new() -> Self {
        Input { cursor_pos: Vec2::ZERO, buttons: InputTracker::new() }
    }

    pub fn update(&mut self) {
        dump!("cursor pos {:.2}", self.cursor_pos);
        self.buttons.update();
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.buttons.handle_event(event);
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = vec2(position.x as f32, position.y as f32);
            }
            _ => (),
        }
    }

    pub fn get_input_model(&self) -> InputModel {
        let mut player_move_direction = Vec2::ZERO;
        if self.buttons.is_key_held(KeyCode::KeyA) {
            player_move_direction += Vec2::NEG_X;
        }
        if self.buttons.is_key_held(KeyCode::KeyW) {
            player_move_direction += Vec2::NEG_Y;
        }
        if self.buttons.is_key_held(KeyCode::KeyD) {
            player_move_direction += Vec2::X;
        }
        if self.buttons.is_key_held(KeyCode::KeyS) {
            player_move_direction += Vec2::Y;
        }
        player_move_direction = player_move_direction.normalize_or_zero();

        let shoot_pressed = self.buttons.is_button_pressed(MouseButton::Left);

        let model = InputModel { player_move_direction, shoot_pressed };
        dump!("input: {model:#.2?}");
        model
    }
}
