use crate::prelude::*;

pub const CIRCLE_FILL_RADIUS: u32 = 16;

#[derive(Debug, Clone, Copy)]
pub struct UiElement {
    pub tint: Color,
    pub ty: UiElementType,
    pub anchoring: Anchoring,
    pub pos: Vec2,
}

impl UiElement {
    pub fn rect(&self) -> UiRect {
        let size = self.ty.size();
        let Vec2 { x: w, y: h } = size;

        // self.pos is the position of the anchor.
        // We, however, want the top-left position.
        let left_top_off: Vec2;
        match self.anchoring {
            Anchoring::Left => left_top_off = vec2(0.0, -h / 2.0),
            Anchoring::LeftBot => left_top_off = vec2(0.0, -h),
            Anchoring::Bot => left_top_off = vec2(-w / 2.0, -h),
            Anchoring::RightBot => left_top_off = vec2(-w, -h),
            Anchoring::Right => left_top_off = vec2(-w, -h / 2.0),
            Anchoring::RightTop => left_top_off = vec2(-w, 0.0),
            Anchoring::Top => left_top_off = vec2(-w / 2.0, 0.0),
            Anchoring::LeftTop => left_top_off = vec2(0.0, 0.0),
        };

        UiRect { left_top: self.pos + left_top_off, size }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchoring {
    Left,
    LeftBot,
    Bot,
    RightBot,
    Right,
    RightTop,
    Top,
    LeftTop,
}

#[derive(Debug, Clone, Copy)]
pub enum UiElementType {
    StackCounter {
        val: u32,
        max_val: u32,
        tex_rect_pos: UVec2,
        tex_rect_size: UVec2,
        direction: StackDirection,
        spacing: f32,
    },
    CircleFill {
        progress: f32,
    },
}

impl UiElementType {
    pub fn size(&self) -> Vec2 {
        match self {
            UiElementType::StackCounter { max_val, tex_rect_size, direction, spacing, .. } => {
                match direction {
                    StackDirection::Left | StackDirection::Right => {
                        let v = *tex_rect_size * uvec2(*max_val, 1);
                        v.as_vec2() + vec2(*spacing, 0.0) * (*max_val as f32)
                    }
                    StackDirection::Down | StackDirection::Up => {
                        let v = *tex_rect_size * uvec2(1, *max_val);
                        v.as_vec2() + vec2(0.0, *spacing) * (*max_val as f32)
                    }
                }
            }
            UiElementType::CircleFill { .. } => (UVec2::splat(CIRCLE_FILL_RADIUS) * 2).as_vec2(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackDirection {
    Left,
    Right,
    Down,
    Up,
}

#[derive(Debug, Clone, Copy)]
pub struct UiRect {
    pub left_top: Vec2,
    pub size: Vec2,
}
