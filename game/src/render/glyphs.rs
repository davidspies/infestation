//! Input prompts: keyboard keycaps and controller buttons drawn in the style
//! of each controller family.

use macroquad::prelude::*;
use quad_gamepad::ControllerType;

use crate::render::palette::{INK, rgb};
use crate::render::shapes::{circle, polyline, ring, rounded_rect, rounded_rect_outline};
use crate::render::text::{self, Align};
use crate::sprites::{Face, Sprites};

/// A physical control on a gamepad.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PadButton {
    South,
    West,
    LeftShoulder,
    RightShoulder,
    Start,
    DPad,
    Stick,
}

/// Something to press, shown as a small picture.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Glyph {
    Key(&'static str),
    /// The four arrow keys as one cluster.
    Arrows,
    /// W A S D as one cluster.
    Wasd,
    Pad(ControllerType, PadButton),
}

/// Width of a glyph drawn `h` pixels tall.
pub(crate) fn width(sprites: &Sprites, glyph: Glyph, h: f32) -> f32 {
    match glyph {
        Glyph::Key(label) => keycap_width(sprites, label, h),
        Glyph::Arrows | Glyph::Wasd => h * 2.15,
        Glyph::Pad(ty, button) => match pad_label(ty, button) {
            PadFace::Pill(label) => h * 0.5 + text::width(sprites, label, Face::Display, h * 0.48),
            _ => h,
        },
    }
}

/// Draw a glyph with its left edge and vertical center at `pos`; returns
/// its width.
pub(crate) fn draw(sprites: &Sprites, glyph: Glyph, pos: Vec2, h: f32) -> f32 {
    let w = width(sprites, glyph, h);
    match glyph {
        Glyph::Key(label) => keycap(sprites, label, Rect::new(pos.x, pos.y - h / 2.0, w, h)),
        Glyph::Arrows => key_cluster(sprites, ["↑", "←", "↓", "→"], pos, h),
        Glyph::Wasd => key_cluster(sprites, ["W", "A", "S", "D"], pos, h),
        Glyph::Pad(ty, button) => pad(sprites, ty, button, Rect::new(pos.x, pos.y - h / 2.0, w, h)),
    }
    w
}

fn keycap_width(sprites: &Sprites, label: &str, h: f32) -> f32 {
    let size = key_font_size(label, h);
    (text::width(sprites, label, Face::Display, size) + h * 0.5).max(h)
}

fn key_font_size(label: &str, h: f32) -> f32 {
    if label.chars().count() > 1 {
        h * 0.42
    } else {
        h * 0.56
    }
}

/// A chunky light keycap with a darker base.
fn keycap(sprites: &Sprites, label: &str, r: Rect) {
    let radius = r.h * 0.22;
    let depth = r.h * 0.12;
    rounded_rect(
        Rect::new(r.x, r.y + depth, r.w, r.h - depth),
        radius,
        rgb(0x8f86a3),
    );
    rounded_rect(Rect::new(r.x, r.y, r.w, r.h - depth), radius, rgb(0xefe9f5));
    rounded_rect_outline(
        Rect::new(r.x, r.y, r.w, r.h),
        radius,
        (r.h * 0.06).max(1.0),
        INK,
    );
    text::draw_aligned(
        sprites,
        label,
        vec2(r.center().x, r.y + (r.h - depth) / 2.0),
        Align::Center,
        if label.chars().any(|c| c.is_ascii_alphanumeric()) {
            Face::Display
        } else {
            Face::Body
        },
        key_font_size(label, r.h),
        INK,
    );
}

/// Four keys in an inverted-T, scaled to fit height `h`.
fn key_cluster(sprites: &Sprites, [up, left, down, right]: [&str; 4], pos: Vec2, h: f32) {
    let k = h * 0.5;
    let gap = h * 0.04;
    let x0 = pos.x;
    let top = pos.y - h / 2.0;
    let key = |label: &str, x: f32, y: f32| keycap(sprites, label, Rect::new(x, y, k * 1.35, k));
    key(up, x0 + k * 1.35 + gap, top);
    key(left, x0, top + k + gap);
    key(down, x0 + k * 1.35 + gap, top + k + gap);
    key(right, x0 + (k * 1.35 + gap) * 2.0, top + k + gap);
}

enum PadFace {
    /// A round face button with a letter, in the button's color.
    Letter(&'static str, Color),
    PsCross,
    PsSquare,
    /// Shoulder or menu buttons, as a labeled pill.
    Pill(&'static str),
    DPad,
    Stick,
}

fn pad_label(ty: ControllerType, button: PadButton) -> PadFace {
    use ControllerType::*;
    use PadButton::*;
    match (ty, button) {
        (_, DPad) => PadFace::DPad,
        (_, Stick) => PadFace::Stick,
        (PlayStation, South) => PadFace::PsCross,
        (PlayStation, West) => PadFace::PsSquare,
        (PlayStation, LeftShoulder) => PadFace::Pill("L1"),
        (PlayStation, RightShoulder) => PadFace::Pill("R1"),
        (PlayStation, Start) => PadFace::Pill("OPTIONS"),
        (Nintendo, South) => PadFace::Letter("B", rgb(0x6e6a7a)),
        (Nintendo, West) => PadFace::Letter("Y", rgb(0x6e6a7a)),
        (Nintendo, LeftShoulder) => PadFace::Pill("L"),
        (Nintendo, RightShoulder) => PadFace::Pill("R"),
        (Nintendo, Start) => PadFace::Pill("+"),
        (Xbox | Generic, South) => PadFace::Letter("A", rgb(0x4caf50)),
        (Xbox | Generic, West) => PadFace::Letter("X", rgb(0x3b82f6)),
        (Xbox | Generic, LeftShoulder) => PadFace::Pill("LB"),
        (Xbox | Generic, RightShoulder) => PadFace::Pill("RB"),
        (Xbox | Generic, Start) => PadFace::Pill("MENU"),
    }
}

fn pad(sprites: &Sprites, ty: ControllerType, button: PadButton, r: Rect) {
    let c = r.center();
    let rad = r.h * 0.46;
    let line = (r.h * 0.09).max(1.5);
    let face_button = || {
        circle(c + vec2(0.0, r.h * 0.05), rad, rgb(0x14111b));
        circle(c, rad, rgb(0x2e2838));
        ring(c, rad, line * 0.7, INK);
    };
    match pad_label(ty, button) {
        PadFace::Letter(label, color) => {
            circle(c + vec2(0.0, r.h * 0.05), rad, darken(color));
            circle(c, rad, color);
            ring(c, rad, line * 0.7, INK);
            text::draw_aligned(
                sprites,
                label,
                c,
                Align::Center,
                Face::Display,
                r.h * 0.58,
                WHITE,
            );
        }
        PadFace::PsCross => {
            face_button();
            let d = rad * 0.45;
            let color = rgb(0x7fb2ff);
            polyline(&[c - vec2(d, d), c + vec2(d, d)], line, color);
            polyline(&[c + vec2(-d, d), c + vec2(d, -d)], line, color);
        }
        PadFace::PsSquare => {
            face_button();
            let d = rad * 0.42;
            rounded_rect_outline(
                Rect::new(c.x - d, c.y - d, d * 2.0, d * 2.0),
                line * 0.4,
                line,
                rgb(0xf38fd8),
            );
        }
        PadFace::Pill(label) => {
            let radius = r.h * 0.4;
            rounded_rect(
                Rect::new(r.x, r.y + r.h * 0.08, r.w, r.h * 0.92),
                radius,
                rgb(0x14111b),
            );
            rounded_rect(Rect::new(r.x, r.y, r.w, r.h * 0.92), radius, rgb(0x3a3346));
            rounded_rect_outline(Rect::new(r.x, r.y, r.w, r.h), radius, line * 0.7, INK);
            text::draw_aligned(
                sprites,
                label,
                vec2(c.x, r.y + r.h * 0.46),
                Align::Center,
                Face::Display,
                r.h * 0.48,
                rgb(0xe9e2f2),
            );
        }
        PadFace::DPad => {
            let arm = r.h * 0.34;
            let len = r.h * 0.5;
            let color = rgb(0x3a3346);
            for (w, h) in [(arm, len * 2.0), (len * 2.0, arm)] {
                rounded_rect(
                    Rect::new(c.x - w / 2.0, c.y - h / 2.0, w, h),
                    arm * 0.25,
                    color,
                );
            }
            for (w, h) in [(arm, len * 2.0), (len * 2.0, arm)] {
                rounded_rect_outline(
                    Rect::new(c.x - w / 2.0, c.y - h / 2.0, w, h),
                    arm * 0.25,
                    line * 0.6,
                    INK,
                );
            }
            for (w, h) in [(arm * 0.8, len * 2.0 - line), (len * 2.0 - line, arm * 0.8)] {
                rounded_rect(
                    Rect::new(c.x - w / 2.0, c.y - h / 2.0, w, h),
                    arm * 0.2,
                    color,
                );
            }
        }
        PadFace::Stick => {
            circle(c, rad, rgb(0x14111b));
            circle(c - vec2(0.0, r.h * 0.05), rad * 0.78, rgb(0x3a3346));
            ring(c - vec2(0.0, r.h * 0.05), rad * 0.78, line * 0.7, INK);
            ring(c, rad, line * 0.7, INK);
        }
    }
}

fn darken(color: Color) -> Color {
    Color::new(color.r * 0.55, color.g * 0.55, color.b * 0.55, color.a)
}
