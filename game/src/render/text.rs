//! Text drawing, rasterized at the display's physical resolution so it
//! stays crisp on high-DPI screens.

use macroquad::prelude::*;

use crate::sprites::{Face, Sprites};

/// Pixel sizes glyphs are rasterized at. Text of any size is drawn by
/// scaling down the next size up, so the glyph cache stays small (it grows
/// with every distinct size, and regrowing mid-frame corrupts glyphs
/// already drawn that frame).
pub(crate) const RASTER_SIZES: [u16; 14] =
    [12, 16, 20, 24, 32, 40, 48, 64, 80, 96, 128, 160, 192, 256];

fn params(sprites: &Sprites, face: Face, size: f32, color: Color) -> TextParams<'_> {
    let physical = size * screen_dpi_scale();
    let raster = RASTER_SIZES
        .iter()
        .copied()
        .find(|&r| r as f32 >= physical)
        .unwrap_or(RASTER_SIZES[RASTER_SIZES.len() - 1]);
    TextParams {
        font: Some(sprites.font(face)),
        font_size: raster,
        font_scale: size / raster as f32,
        color,
        ..Default::default()
    }
}

/// Width of `text` in logical pixels.
pub(crate) fn width(sprites: &Sprites, text: &str, face: Face, size: f32) -> f32 {
    let p = params(sprites, face, size, WHITE);
    measure_text(text, p.font, p.font_size, p.font_scale).width
}

/// Height of a capital letter, for vertically centering text.
pub(crate) fn cap_height(face: Face, size: f32) -> f32 {
    size * match face {
        Face::Display => 0.70,
        Face::Body => 0.64,
    }
}

/// Draw with the left end of the baseline at `pos`.
pub(crate) fn draw(sprites: &Sprites, text: &str, pos: Vec2, face: Face, size: f32, color: Color) {
    draw_text_ex(text, pos.x, pos.y, params(sprites, face, size, color));
}

/// Where text starts horizontally for the given alignment.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Align {
    Left,
    Center,
    Right,
}

/// Draw `text` with its capitals vertically centered on `anchor.y`, aligned
/// horizontally to `anchor.x`.
pub(crate) fn draw_aligned(
    sprites: &Sprites,
    text: &str,
    anchor: Vec2,
    align: Align,
    face: Face,
    size: f32,
    color: Color,
) {
    let w = width(sprites, text, face, size);
    let x = match align {
        Align::Left => anchor.x,
        Align::Center => anchor.x - w / 2.0,
        Align::Right => anchor.x - w,
    };
    let y = anchor.y + cap_height(face, size) / 2.0;
    draw(sprites, text, vec2(x, y), face, size, color);
}

/// Centered display text with a chunky dark outline and drop shadow, for
/// titles and banners.
pub(crate) fn draw_title(
    sprites: &Sprites,
    text: &str,
    center: Vec2,
    size: f32,
    color: Color,
    outline: Color,
) {
    let w = width(sprites, text, Face::Display, size);
    let base = vec2(
        center.x - w / 2.0,
        center.y + cap_height(Face::Display, size) / 2.0,
    );
    let t = (size * 0.07).max(1.5);
    draw(
        sprites,
        text,
        base + vec2(0.0, t * 1.6),
        Face::Display,
        size,
        Color {
            a: outline.a * 0.6,
            ..outline
        },
    );
    for i in 0..12 {
        let a = i as f32 / 12.0 * std::f32::consts::TAU;
        draw(
            sprites,
            text,
            base + vec2(a.cos(), a.sin()) * t,
            Face::Display,
            size,
            outline,
        );
    }
    draw(sprites, text, base, Face::Display, size, color);
}

/// Break `text` into lines no wider than `max_width`, keeping explicit
/// newlines.
pub(crate) fn wrap(
    sprites: &Sprites,
    text: &str,
    face: Face,
    size: f32,
    max_width: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && width(sprites, &candidate, face, size) > max_width {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}
