use std::rc::Rc;

use bytemuck::{Pod, Zeroable};
use mimiq::UniformBlock;
use mimiq::Vertex;
use mimiq::glam::*;
use mimiq::graphics::*;
use mimiq::util::{BasicTexImages, GeometryBatcher};

#[derive(Debug, Clone, Copy)]
pub struct RenderSprite {
    pub color: Color,
    pub tex_rect_pos: UVec2,
    pub tex_rect_size: UVec2,
    pub transform: Affine2,
}

pub struct SpriteBatcher(pub GeometryBatcher<SpriteVertex>);

impl SpriteBatcher {
    pub fn new_from_size(ctx: &Rc<GlContext>, sprites: usize) -> Result<Self> {
        let inner = GeometryBatcher::new_from_size(ctx, sprites, sprites)?;
        Ok(SpriteBatcher(inner))
    }

    pub fn new(batcher: GeometryBatcher<SpriteVertex>) -> Self {
        SpriteBatcher(batcher)
    }

    #[track_caller]
    pub fn add_sprite(&mut self, sprite: RenderSprite) {
        const INDICIES: &[u16] = &[0, 1, 2, 0, 2, 3];

        let tf = sprite.transform;

        // Verts
        let center = tf.transform_point2(Vec2::ZERO);
        let halfs = sprite.tex_rect_size.as_vec2() * vec2(0.5, 0.5);
        let horizontal = tf.transform_vector2(vec2(halfs.x, 0.0));
        let vertical = tf.transform_vector2(vec2(0.0, halfs.y));

        // Texcoords
        let tex_top_left = sprite.tex_rect_pos.as_vec2();
        let Vec2 { x: tex_width, y: tex_height } = sprite.tex_rect_size.as_vec2();

        // (-1.0, 1.0)
        let p1 = center - horizontal + vertical;
        let t1 = tex_top_left + vec2(0.0, tex_height);
        // (1.0,-1.0)
        let p2 = center + horizontal + vertical;
        let t2 = tex_top_left + vec2(tex_width, tex_height);
        // (1.0, -1.0)
        let p3 = center + horizontal - vertical;
        let t3 = tex_top_left + vec2(tex_width, 0.0);
        // (-1.0, -1.0)
        let p4 = center - horizontal - vertical;
        let t4 = tex_top_left;

        let vertices = &[
            SpriteVertex { v_pos: p1, v_uv_not_normalized: t1, v_color: sprite.color },
            SpriteVertex { v_pos: p2, v_uv_not_normalized: t2, v_color: sprite.color },
            SpriteVertex { v_pos: p3, v_uv_not_normalized: t3, v_color: sprite.color },
            SpriteVertex { v_pos: p4, v_uv_not_normalized: t4, v_color: sprite.color },
        ];

        self.0.extend(vertices, INDICIES);
    }

    pub fn flush(&mut self) -> u32 {
        self.0.flush()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }
}

#[derive(Debug, Default, Pod, Zeroable, Clone, Copy, Vertex)]
#[repr(C)]
pub struct SpriteVertex {
    pub v_pos: Vec2,
    pub v_uv_not_normalized: Vec2,
    pub v_color: Color,
}

pub type SpritePipeline = Pipeline<SpriteVertex, SpritePipelineUniforms, BasicTexImages<'static>>;

pub fn new_sprite_pipeline(ctx: &Rc<GlContext>) -> Result<SpritePipeline> {
    ctx.new_pipeline(
        include_str!("shaders/sprite.vert"),
        include_str!("shaders/sprite.frag"),
        PipelineParams {
            blending: Blending::All(BlendFunc {
                equation: BlendEquation::Add,
                source: BlendFactor::Value(BlendValue::SrcAlpha),
                dest: BlendFactor::OneMinusValue(BlendValue::SrcAlpha),
            }),
            ..default_pipeline_params()
        },
    )
}

#[repr(C)]
#[derive(Debug, Pod, Zeroable, Clone, Copy, UniformBlock)]
pub struct SpritePipelineUniforms {
    pub view_projection: Mat4,
    pub width_height: Vec2,
}
