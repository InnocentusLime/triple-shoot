pub use crate::collisions::components::*;
pub use crate::render::components::*;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PlayerTag;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ProjectileTag {
    pub speed: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub time_left: f32,
}

impl Lifetime {
    pub fn from_time(time: f32) -> Self {
        Lifetime { time_left: time }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Hp {
    pub cooldown_length: f32,
    pub hp: i32,
    #[serde(skip)]
    pub cooldown: f32,
}

impl Hp {
    pub fn cooling_down(&self) -> bool {
        self.cooldown > 0.0
    }

    pub fn damage(&mut self, delta: i32) {
        if self.cooling_down() {
            return;
        }
        self.hp -= delta;
        self.cooldown = self.cooldown_length;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub pos: Vec2,
    pub angle: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self { pos: Vec2::ZERO, angle: 0.0 };

    pub fn from_unit(_: ()) -> Self {
        Self::IDENTITY
    }

    pub fn from_pos(pos: Vec2) -> Self {
        Self { pos, angle: 0.0 }
    }

    pub fn from_xy(x: f32, y: f32) -> Self {
        Self::from_pos(vec2(x, y))
    }
}
