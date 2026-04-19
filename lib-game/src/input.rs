use crate::components::*;
use crate::prelude::*;

use mimiq::util::InputTracker;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum WeaponId {
    Shotgun,
    Rifle,
}

#[derive(Debug)]
pub struct InputModel {
    pub player_move_direction: Vec2,
    pub shoot_down: bool,
    pub player_aim_direction: Vec2,
    pub player_weapon_request: Option<WeaponId>,
}

pub struct Input {
    screen_to_world: Mat3,
    /// tracks the mouse cursor position in world coordinates.
    cursor_pos: Vec2,
    buttons: InputTracker,
}

impl Input {
    pub fn new() -> Self {
        Input {
            cursor_pos: Vec2::ZERO,
            buttons: InputTracker::new(),
            screen_to_world: Mat3::IDENTITY,
        }
    }

    pub fn update(&mut self) {
        dump!("cursor pos {:.2}", self.cursor_pos);
        self.buttons.update();
    }

    pub fn handle_event(&mut self, event: &WindowEvent) {
        self.buttons.handle_event(event);
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let cursor_pos = Vec2::from_array((*position).into());
                self.cursor_pos = self.screen_to_world.transform_point2(cursor_pos);
            }
            WindowEvent::Resized(native) => {
                self.screen_to_world =
                    crate::resolution::native_to_screen(native.width, native.height);
            }
            _ => (),
        }
    }

    pub fn get_input_model(&self, world: &mut World) -> InputModel {
        static KEY_AND_WEAPONS: [(KeyCode, WeaponId); 2] =
            [(KeyCode::Digit1, WeaponId::Shotgun), (KeyCode::Digit2, WeaponId::Rifle)];

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

        let shoot_pressed = self.buttons.is_button_held(MouseButton::Left);

        let mut player_aim_direction = Vec2::Y;
        for (_, player_tf) in world.query_mut::<&Transform>().with::<&PlayerTag>() {
            let dr = self.cursor_pos - player_tf.pos;
            player_aim_direction = dr.normalize_or(player_aim_direction);
        }

        let player_weapon_request = KEY_AND_WEAPONS
            .into_iter()
            .find(|(key, _)| self.buttons.is_key_pressed(*key))
            .map(|(_, x)| x);

        let model = InputModel {
            player_move_direction,
            shoot_down: shoot_pressed,
            player_aim_direction,
            player_weapon_request,
        };
        dump!("input: {model:#.2?}");
        model
    }
}
