//! Graphics assets: the sprite atlas, tiling textures, fonts and blend
//! materials, plus the primitive for drawing an atlas sprite.

use enum_map::EnumMap;
use macroquad::miniquad::{
    BlendFactor, BlendState, BlendValue, Equation, FilterMode, MipmapFilterMode, PipelineParams,
    TextureWrap,
};
use macroquad::prelude::*;

use crate::atlas::{Atlas, SpriteId};

/// Same as macroquad's default sprite shader; paired with an additive
/// blend state for glows and sparks.
const VERTEX_SHADER: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const FRAGMENT_SHADER: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;

uniform sampler2D Texture;

void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

/// Fonts used for all text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Face {
    /// Chunky rounded display face for titles, buttons and numbers.
    Display,
    /// Readable book face for running text.
    Body,
}

#[derive(Clone)]
pub struct Sprites {
    atlas: Texture2D,
    rects: EnumMap<SpriteId, Rect>,
    /// Floor flagstones: a 4x4 grid of interchangeable cell tiles.
    pub(crate) floor: Texture2D,
    /// Top surface of walls, sampled world-aligned (4x4 cells per repeat).
    pub(crate) wall_top: Texture2D,
    /// South face of walls, repeating horizontally every 4 cells.
    pub(crate) wall_front: Texture2D,
    display_font: Font,
    body_font: Font,
    additive: Material,
    /// Writes only alpha, to make the frame fully opaque.
    alpha_fill: Material,
}

/// Decode an RGB JPEG into a texture.
fn load_jpeg(bytes: &[u8]) -> Texture2D {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    let pixels = decoder.decode().expect("bundled JPEG should decode");
    let info = decoder.info().expect("decoded JPEG has its header");
    assert_eq!(info.pixel_format, jpeg_decoder::PixelFormat::RGB24);
    let rgba: Vec<u8> = pixels
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[r, g, b]| [r, g, b, 255])
        .collect();
    Texture2D::from_rgba8(info.width, info.height, &rgba)
}

/// Make a texture smooth and mipmapped (power-of-two sizes only, for
/// WebGL 1).
fn mipmapped(texture: Texture2D, wrap: TextureWrap) -> Texture2D {
    let (w, h) = (texture.width() as u32, texture.height() as u32);
    assert!(
        w.is_power_of_two() && h.is_power_of_two(),
        "mipmapped textures must be power-of-two sized, got {w}x{h}"
    );
    let id = texture.raw_miniquad_id();
    let ctx = unsafe { get_internal_gl() }.quad_context;
    ctx.texture_set_wrap(id, wrap, wrap);
    ctx.texture_set_filter(id, FilterMode::Linear, MipmapFilterMode::Linear);
    ctx.texture_generate_mipmaps(id);
    texture
}

fn load_font(bytes: &[u8]) -> Font {
    let mut font = load_ttf_font_from_bytes(bytes).expect("bundled font should parse");
    font.set_filter(FilterMode::Linear);
    // Rasterize common glyphs at the usual sizes up front, so the glyph
    // cache does its growing before the first frame.
    let chars: Vec<char> = (' '..='~').chain("×·—…’←↑→↓".chars()).collect();
    for &size in crate::render::text::RASTER_SIZES
        .iter()
        .filter(|&&s| s <= 64)
    {
        font.populate_font_cache(&chars, size);
    }
    font
}

impl Sprites {
    pub async fn load() -> Self {
        let atlas = Atlas::parse(include_str!("../../assets/sprites.json"));
        let texture = mipmapped(
            Texture2D::from_file_with_format(
                include_bytes!("../../assets/sprites.png"),
                Some(ImageFormat::Png),
            ),
            TextureWrap::Clamp,
        );
        assert_eq!(
            (texture.width() as u32, texture.height() as u32),
            atlas.size,
            "sprite atlas image and JSON disagree on size"
        );
        let additive = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("additive shader should compile");
        let alpha_fill = load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_write: (false, false, false, true),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("alpha fill shader should compile");
        Self {
            atlas: texture,
            rects: atlas.rects,
            floor: mipmapped(
                load_jpeg(include_bytes!("../../assets/textures/floor.jpg")),
                TextureWrap::Clamp,
            ),
            wall_top: mipmapped(
                load_jpeg(include_bytes!("../../assets/textures/wall_top.jpg")),
                TextureWrap::Repeat,
            ),
            wall_front: mipmapped(
                load_jpeg(include_bytes!("../../assets/textures/wall_front.jpg")),
                TextureWrap::Repeat,
            ),
            display_font: load_font(include_bytes!("../../assets/fonts/LilitaOne-Regular.ttf")),
            body_font: load_font(include_bytes!(
                "../../assets/fonts/AlegreyaSans-Regular.ttf"
            )),
            additive,
            alpha_fill,
        }
    }

    pub(crate) fn font(&self, face: Face) -> &Font {
        match face {
            Face::Display => &self.display_font,
            Face::Body => &self.body_font,
        }
    }

    /// Draw a sprite centered at `center`, scaled to `size` and rotated
    /// clockwise by `rotation` radians around its center.
    pub(crate) fn draw(&self, id: SpriteId, center: Vec2, size: Vec2, rotation: f32, color: Color) {
        draw_texture_ex(
            &self.atlas,
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            color,
            DrawTextureParams {
                dest_size: Some(size),
                source: Some(self.rects[id]),
                rotation,
                ..Default::default()
            },
        );
    }

    /// Draw a sprite centered at `center` with a uniform `size`, unrotated.
    pub(crate) fn draw_at(&self, id: SpriteId, center: Vec2, size: f32, color: Color) {
        self.draw(id, center, Vec2::splat(size), 0.0, color);
    }

    /// Make the whole frame opaque. Translucent drawing lowers the
    /// framebuffer's alpha, which browsers would composite against the page.
    pub(crate) fn finish_frame(&self) {
        gl_use_material(&self.alpha_fill);
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), WHITE);
        gl_use_default_material();
    }

    /// Run `draw` with additive blending (for light: glows, sparks, flashes).
    pub(crate) fn additive(&self, draw: impl FnOnce()) {
        gl_use_material(&self.additive);
        draw();
        gl_use_default_material();
    }
}
