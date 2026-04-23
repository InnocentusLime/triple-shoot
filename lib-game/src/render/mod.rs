pub mod components;

use crate::components::*;
use crate::prelude::*;

use bytemuck::{Pod, Zeroable};
use mimiq::util::{ShapeBatcher, SpriteBatcher};
use mimiq::{
    BLACK, BLUE, BufferUsage, Clear, DrawCall, PipelineMeta, PipelineParams, Texture2D,
    UniformBlock, UniformField, Vertex, VertexField, attribute_of, default_pipeline_params,
    uniform_of,
};

pub struct Render {
    pub curr_texture: AssetKey,
    pub sprite_batcher: SpriteBatcher,

    pub gizmos: ShapeBatcher,

    pub render_world: bool,
    pub debug_draws: HashMap<String, fn(&mut World, &mut ShapeBatcher)>,
    pub enabled_debug_draws: HashSet<String>,

    pub gamescreen_verts: mimiq::VertexBuffer<QuadVert>,
    pub gamescreen_indicies: mimiq::IndexBuffer,
    pub gamescreen_pipeline: mimiq::Pipeline<PixelPerfectPipelineMeta>,
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
            sprite_batcher: SpriteBatcher::new_from_size(&resources.gl_ctx, 1_000),
            gizmos: ShapeBatcher::new_from_size(&resources.gl_ctx, 20_000, 20_000),
            gamescreen_verts: resources.gl_ctx.new_vertex_buffer(
                BufferUsage::Dynamic,
                &[
                    QuadVert { pos: vec2(-1.0, -1.0), uv: vec2(0.0, 0.0) },
                    QuadVert { pos: vec2(1.0, -1.0), uv: vec2(1.0, 0.0) },
                    QuadVert { pos: vec2(1.0, 1.0), uv: vec2(1.0, 1.0) },
                    QuadVert { pos: vec2(-1.0, 1.0), uv: vec2(0.0, 1.0) },
                ],
            ),
            gamescreen_indicies: resources
                .gl_ctx
                .new_index_buffer(BufferUsage::Immutable, &[0, 1, 2, 0, 2, 3]),
            gamescreen_pipeline: resources.gl_ctx.new_pipeline(),
            render_world: true,
            debug_draws,
            enabled_debug_draws: HashSet::new(),
        }
    }

    pub fn render(&mut self, resources: &mut Resources) {
        self.buffer_sprites(&mut resources.world);
        for debug_draw_name in self.enabled_debug_draws.iter() {
            let ddraw = self.debug_draws[debug_draw_name];
            ddraw(&mut resources.world, &mut self.gizmos);
        }

        resources
            .gamescreen
            .pass(Clear::depth_color(BLUE), |width, height| {
                let view_projection =
                    Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, 0.0, 100.0);
                if self.render_world {
                    self.draw_sprites(resources, view_projection);
                }

                self.gizmos.basic_draw(
                    &resources.gl_ctx,
                    view_projection,
                    &resources.basic_pipeline,
                );
            });

        resources
            .gl_ctx
            .default_pass(Clear::depth_color(BLACK), |width, height| {
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
                resources.gl_ctx.draw(DrawCall {
                    pipeline: &self.gamescreen_pipeline,
                    base_element: 0,
                    num_elements: 6,
                    vertex_buffer: &self.gamescreen_verts,
                    index_buffer: &self.gamescreen_indicies,
                    images: &resources.gamescreen.color_attachments()[0],
                    uniforms: &PixelPerfectUniforms {
                        res: vec2(width as f32, height as f32),
                        view_projection,
                    },
                });
            });
    }

    fn draw_sprites(&mut self, resources: &Resources, view_projection: Mat4) {
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
            return;
        };

        self.sprite_batcher.draw(
            &resources.gl_ctx,
            view_projection,
            &resources.sprite_pipeline,
            texture,
        );
    }

    pub fn buffer_sprites(&mut self, world: &mut World) {
        const FLICKER_INTERVAL: f32 = 0.1;

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
            self.sprite_batcher.add_sprite(mimiq::util::Sprite {
                tex_rect_pos: sprite.tex_rect_pos,
                tex_rect_size: sprite.tex_rect_size,
                color,
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
#[derive(Debug, Default, Pod, Zeroable, Clone, Copy)]
pub struct PixelPerfectUniforms {
    pub res: Vec2,
    pub view_projection: Mat4,
}

impl UniformBlock for PixelPerfectUniforms {
    const FIELDS: &'static [UniformField] = &[
        uniform_of!(PixelPerfectUniforms, res),
        uniform_of!(PixelPerfectUniforms, view_projection),
    ];
}

#[repr(C)]
#[derive(Debug, Default, Pod, Zeroable, Clone, Copy)]
pub struct QuadVert {
    pub pos: Vec2,
    pub uv: Vec2,
}

impl Vertex for QuadVert {
    const LAYOUT: &'static [VertexField] =
        &[attribute_of!(QuadVert, pos), attribute_of!(QuadVert, uv)];
}

pub struct PixelPerfectPipelineMeta;

impl PipelineMeta for PixelPerfectPipelineMeta {
    const VERTEX_SHADER: &str = include_str!("shaders/pixel_perfect.vert");
    const FRAGMENT_SHADER: &str = include_str!("shaders/pixel_perfect.frag");

    const IMAGES_NAMES: &str = "tex";
    type Images = Texture2D;
    type Vertex = QuadVert;
    type Uniforms = PixelPerfectUniforms;
    const PARAMS: PipelineParams = default_pipeline_params();
}
