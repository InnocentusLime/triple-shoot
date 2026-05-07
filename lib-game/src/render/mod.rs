pub mod components;

mod sprites;

use crate::components::*;
use crate::prelude::*;
use crate::ui::*;

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use mimiq::UniformBlock;
use mimiq::Vertex;
use mimiq::graphics::*;
use mimiq::util;
use mimiq::util::{BasicPipeline, ShapeBatcher};

use sprites::*;

pub struct Render {
    pub curr_texture: AssetKey,
    pub sprite_batcher: SpriteBatcher,

    pub gizmos: ShapeBatcher,

    pub render_world: bool,
    pub debug_draws: HashMap<String, fn(&mut World, &mut ShapeBatcher)>,
    pub enabled_debug_draws: HashSet<String>,

    pub gamescreen_verts: VertexBuffer<QuadVert>,
    pub gamescreen_indicies: IndexBuffer,
    pub quad_verts: VertexBuffer<QuadVert>,

    pub ui_elements: Vec<UiElement>,
    pub basic_elements_batcher: SpriteBatcher,

    pub sprite_pipeline: SpritePipeline,
    pub basic_pipeline: BasicPipeline,
    pub gamescreen_pipeline:
        Pipeline<QuadVert, PixelPerfectUniforms, util::BasicTexImages<'static>>,
    pub circle_fill_pipeline: Pipeline<QuadVert, SpinnerUniforms, util::BasicTexImages<'static>>,
}

impl Render {
    pub fn new(resources: &Resources) -> Self {
        let mut debug_draws = HashMap::<String, fn(&mut World, &mut ShapeBatcher)>::new();
        debug_draws.insert(
            "phys".to_string(),
            crate::collisions::debug::draw_physics_debug,
        );

        Self {
            curr_texture: INVALID_ASSET,
            sprite_batcher: SpriteBatcher::new_from_size(&resources.gl_ctx, 1_000).unwrap(),
            gizmos: ShapeBatcher::new_from_size(&resources.gl_ctx, 20_000, 20_000).unwrap(),
            gamescreen_verts: resources
                .gl_ctx
                .new_vertex_buffer(
                    BufferUsage::Dynamic,
                    &[
                        QuadVert { pos: vec2(-1.0, -1.0), uv: vec2(0.0, 0.0) },
                        QuadVert { pos: vec2(1.0, -1.0), uv: vec2(1.0, 0.0) },
                        QuadVert { pos: vec2(1.0, 1.0), uv: vec2(1.0, 1.0) },
                        QuadVert { pos: vec2(-1.0, 1.0), uv: vec2(0.0, 1.0) },
                    ],
                )
                .unwrap(),
            gamescreen_indicies: resources
                .gl_ctx
                .new_index_buffer(BufferUsage::Immutable, &[0, 1, 2, 0, 2, 3])
                .unwrap(),
            render_world: true,
            debug_draws,
            enabled_debug_draws: HashSet::new(),
            quad_verts: resources
                .gl_ctx
                .new_vertex_buffer(
                    BufferUsage::Immutable,
                    &[
                        QuadVert { pos: vec2(-1.0, -1.0), uv: vec2(0.0, 0.0) },
                        QuadVert { pos: vec2(1.0, -1.0), uv: vec2(1.0, 0.0) },
                        QuadVert { pos: vec2(1.0, 1.0), uv: vec2(1.0, 1.0) },
                        QuadVert { pos: vec2(-1.0, 1.0), uv: vec2(0.0, 1.0) },
                    ],
                )
                .unwrap(),
            ui_elements: Vec::new(),
            basic_elements_batcher: SpriteBatcher::new_from_size(&resources.gl_ctx, 100).unwrap(),

            sprite_pipeline: new_sprite_pipeline(&resources.gl_ctx).unwrap(),
            basic_pipeline: util::new_basic_pipeline(&resources.gl_ctx).unwrap(),
            gamescreen_pipeline: resources
                .gl_ctx
                .new_pipeline(
                    include_str!("shaders/pixel_perfect.vert"),
                    include_str!("shaders/pixel_perfect.frag"),
                    default_pipeline_params(),
                )
                .unwrap(),
            circle_fill_pipeline: resources
                .gl_ctx
                .new_pipeline(
                    include_str!("shaders/spinn.vert"),
                    include_str!("shaders/spinn.frag"),
                    PipelineParams {
                        blending: Blending::All(BlendFunc {
                            equation: BlendEquation::Add,
                            source: BlendFactor::Value(BlendValue::SrcAlpha),
                            dest: BlendFactor::OneMinusValue(BlendValue::SrcAlpha),
                        }),
                        ..default_pipeline_params()
                    },
                )
                .unwrap(),
        }
    }

    pub fn render(&mut self, resources: &mut Resources) -> anyhow::Result<()> {
        self.buffer_sprites(&mut resources.world);
        for debug_draw_name in self.enabled_debug_draws.iter() {
            let ddraw = self.debug_draws[debug_draw_name];
            ddraw(&mut resources.world, &mut self.gizmos);
        }

        resources
            .gamescreen
            .pass(Clear::depth_color(Color::BLUE), |width, height| {
                let view_projection =
                    Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, 0.0, 100.0);
                if self.render_world {
                    self.draw_sprites(resources, view_projection)?;
                }

                self.draw_ui_elements(resources, view_projection)?;

                let num_elements = self.gizmos.flush();
                self.basic_pipeline.draw(
                    0,
                    num_elements,
                    &self.gizmos.0.vertices,
                    &self.gizmos.0.indicies,
                    &NoImages,
                    &util::BasicPipelineUniforms { view_projection },
                )?;
                self.gizmos.clear();

                Ok(())
            })
            .context("world render")?;

        resources
            .gl_ctx
            .default_pass(Clear::depth_color(Color::BLACK), |width, height| {
                let (left, right, top, bottom) =
                    crate::resolution::native_scaled_quad_points(width, height);
                self.gamescreen_verts.update(&[
                    QuadVert { pos: vec2(left, bottom), uv: vec2(0.0, 0.0) },
                    QuadVert { pos: vec2(right, bottom), uv: vec2(1.0, 0.0) },
                    QuadVert { pos: vec2(right, top), uv: vec2(1.0, 1.0) },
                    QuadVert { pos: vec2(left, top), uv: vec2(0.0, 1.0) },
                ]);

                dump!("Default pass dimensions: ({width}, {height})");
                let view_projection =
                    Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, 0.0, 100.0);
                self.gamescreen_pipeline.draw(
                    0,
                    6,
                    &self.gamescreen_verts,
                    &self.gamescreen_indicies,
                    &util::BasicTexImages { tex: &resources.gamescreen.color_attachments()[0] },
                    &PixelPerfectUniforms {
                        res: vec2(width as f32, height as f32),
                        view_projection,
                    },
                )?;

                Ok(())
            })
            .context("screen render")?;

        Ok(())
    }

    fn draw_sprites(&mut self, resources: &Resources, view_projection: Mat4) -> Result<()> {
        // TODO: need sprite length
        // dump!("sprites drawn: {}", self.sprite_buffer.len());

        // TODO: need possibility to sort
        // self.sprite_batcher.sort_by(|s1, s2| {
        //     let y_s1 = s1.tf.pos.y + s1.sort_offset;
        //     let y_s2 = s2.tf.pos.y + s2.sort_offset;
        //     u32::cmp(&s1.layer, &s2.layer).then(f32::total_cmp(&y_s1, &y_s2))
        // });

        let Some(texture) = resources.textures.get(self.curr_texture) else {
            // warn!("No texture {:?}", sprite.texture);
            return Ok(());
        };

        let num_elements = self.sprite_batcher.flush();
        self.sprite_pipeline.draw(
            0,
            num_elements,
            &self.sprite_batcher.0.vertices,
            &self.sprite_batcher.0.indicies,
            &util::BasicTexImages { tex: texture },
            &SpritePipelineUniforms { view_projection, width_height: texture.size().as_vec2() },
        )
    }

    fn draw_ui_elements(&mut self, resources: &Resources, view_projection: Mat4) -> Result<()> {
        self.basic_elements_batcher.clear();

        let Some(ui_texture) = resources.textures.resolve("atlas/ui.png") else {
            return Ok(());
        };
        let Some(ui_texture) = resources.textures.get(ui_texture) else {
            return Ok(());
        };
        let Some(grad_texture) = resources.textures.resolve("atlas/grad.png") else {
            return Ok(());
        };
        let Some(grad_texture) = resources.textures.get(grad_texture) else {
            return Ok(());
        };

        for element in &self.ui_elements {
            let rect = element.rect();
            match element.ty {
                UiElementType::StackCounter {
                    val,
                    tex_rect_pos,
                    tex_rect_size,
                    direction,
                    spacing,
                    ..
                } => Self::buffer_stack(
                    &mut self.basic_elements_batcher,
                    element.tint,
                    rect,
                    val,
                    spacing,
                    direction,
                    tex_rect_pos,
                    tex_rect_size,
                ),
                _ => (),
            }
        }

        let num_elements = self.basic_elements_batcher.flush();
        self.sprite_pipeline.draw(
            0,
            num_elements,
            &self.basic_elements_batcher.0.vertices,
            &self.basic_elements_batcher.0.indicies,
            &util::BasicTexImages { tex: ui_texture },
            &SpritePipelineUniforms { view_projection, width_height: ui_texture.size().as_vec2() },
        )?;

        for element in &self.ui_elements {
            let rect = element.rect();
            match element.ty {
                UiElementType::CircleFill { progress } => {
                    self.draw_circle_fill(rect, progress, view_projection, grad_texture)?
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn buffer_stack(
        batcher: &mut SpriteBatcher,
        tint: Color,
        rect: UiRect,
        val: u32,
        spacing: f32,
        direction: StackDirection,
        tex_rect_pos: UVec2,
        tex_rect_size: UVec2,
    ) {
        let size = tex_rect_size.as_vec2();
        let Vec2 { x: half_w, y: half_h } = size / 2.0;

        let start: Vec2;
        let step: Vec2;
        match direction {
            StackDirection::Left => {
                start = rect.left_top + rect.size + vec2(-half_w, -half_h);
                step = vec2(-size.x + -spacing, 0.0);
            }
            StackDirection::Right => {
                start = rect.left_top + vec2(half_w, half_h);
                step = vec2(size.x + spacing, 0.0);
            }
            StackDirection::Down => {
                start = rect.left_top + vec2(half_w, half_h);
                step = vec2(0.0, size.y + spacing);
            }
            StackDirection::Up => {
                start = rect.left_top + rect.size + vec2(-half_w, -half_h);
                step = vec2(0.0, -size.y + -spacing);
            }
        };

        dump!("start: {start:.2}");
        for idx in 0..val {
            let pos = start + step * idx as f32;
            batcher.add_sprite(RenderSprite {
                color: tint,
                tex_rect_pos,
                tex_rect_size,
                transform: Affine2::from_translation(pos),
            });
        }
    }

    fn draw_circle_fill(
        &self,
        rect: UiRect,
        progress: f32,
        view_projection: Mat4,
        grad_texture: &Texture2D,
    ) -> Result<()> {
        let scale = grad_texture.size().as_vec2() / 2.0;
        let model = Mat4::from_scale_rotation_translation(
            scale.extend(1.0),
            Quat::IDENTITY,
            (rect.left_top + rect.size / 2.0).extend(0.0),
        );

        self.circle_fill_pipeline.draw(
            0,
            6,
            &self.quad_verts,
            &self.gamescreen_indicies,
            &util::BasicTexImages { tex: &grad_texture },
            &SpinnerUniforms { view_projection: view_projection * model, progress },
        )
    }

    pub fn buffer_sprites(&mut self, world: &mut World) {
        const FLICKER_INTERVAL: f32 = 0.1;

        self.sprite_batcher.clear();
        for (_, (tf, sprite, hp)) in world.query_mut::<(&Transform, &Sprite, Option<&Hp>)>() {
            let pos = tf.pos + sprite.local_offset;
            let transform = Affine2::from_angle_translation(tf.angle, pos);
            let mut color = sprite.color;
            if let Some(hp) = hp
                && hp.cooling_down()
            {
                let interval = hp.cooldown.div_euclid(FLICKER_INTERVAL) as u32;
                if interval % 2 == 0 {
                    color.a = 0.0;
                }
            }

            self.curr_texture = sprite.texture;
            self.sprite_batcher.add_sprite(RenderSprite {
                color,
                tex_rect_pos: sprite.tex_rect_pos,
                tex_rect_size: sprite.tex_rect_size,
                transform,
            });
        }
    }
}

// #[derive(Debug, Clone, Copy)]
// pub struct SpriteData {
//     pub layer: u32,
//     pub tf: Transform,
//     pub texture: AssetKey,
//     pub rect: Rect,
//     pub color: Color,
//     pub sort_offset: f32,
// }

#[repr(C)]
#[derive(Debug, Default, Pod, Zeroable, Clone, Copy, UniformBlock)]
pub struct PixelPerfectUniforms {
    pub res: Vec2,
    pub view_projection: Mat4,
}

#[repr(C)]
#[derive(Debug, Default, Pod, Zeroable, Clone, Copy, Vertex)]
pub struct QuadVert {
    pub pos: Vec2,
    pub uv: Vec2,
}

#[repr(C)]
#[derive(Debug, Pod, Zeroable, Clone, Copy, UniformBlock)]
pub struct SpinnerUniforms {
    pub view_projection: Mat4,
    pub progress: f32,
}
