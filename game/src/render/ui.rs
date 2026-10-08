//! Screen layout and shared widgets: panels, cards and buttons.

use macroquad::prelude::*;

use crate::atlas::SpriteId;
use crate::render::glyphs::{self, Glyph};
use crate::render::palette::{GOLD, INK, PANEL, PANEL_EDGE, PANEL_LIGHT, TEXT, TEXT_DIM, rgb};
use crate::render::shapes::{
    faded, rounded_rect, rounded_rect_gradient, rounded_rect_outline, soft_shadow,
};
use crate::render::text::{self, Align};
use crate::sprites::{Face, Sprites};

/// How the screen is divided between the play area and the side panel.
#[derive(Clone, Copy)]
pub(crate) struct ScreenLayout {
    /// The board or map.
    pub(crate) main: Rect,
    /// Sidebar (landscape) or bottom panel (portrait).
    pub(crate) panel: Rect,
    /// Title bar across the top (portrait only).
    pub(crate) top: Option<Rect>,
    /// UI scale factor: sizes below are nominal pixels times this.
    pub(crate) s: f32,
}

impl ScreenLayout {
    pub(crate) fn current() -> Self {
        let (w, h) = (screen_width(), screen_height());
        if w < h * 1.05 {
            let s = (w / 400.0).clamp(0.8, 1.6);
            let m = 8.0 * s;
            let top_h = 52.0 * s;
            let panel_h = (196.0 * s).min(h * 0.4);
            let top = Rect::new(m, m, w - 2.0 * m, top_h);
            let panel = Rect::new(m, h - panel_h - m, w - 2.0 * m, panel_h);
            let main = Rect::new(
                m,
                top.bottom() + m,
                w - 2.0 * m,
                panel.y - top.bottom() - 2.0 * m,
            );
            Self {
                main,
                panel,
                top: Some(top),
                s,
            }
        } else {
            let s = (h / 820.0).clamp(0.75, 1.7).min(w / 1100.0);
            let m = 14.0 * s;
            let panel_w = (w * 0.3).clamp(270.0 * s, 380.0 * s);
            let panel = Rect::new(w - panel_w - m, m, panel_w, h - 2.0 * m);
            let main = Rect::new(m, m, panel.x - 2.0 * m, h - 2.0 * m);
            Self {
                main,
                panel,
                top: None,
                s,
            }
        }
    }

    pub(crate) fn portrait(&self) -> bool {
        self.top.is_some()
    }
}

/// The mouse position, if it's hovering (not on touch screens).
pub(crate) fn hover_pos() -> Option<Vec2> {
    (!quad_touch::is_touch_device()).then(|| Vec2::from(mouse_position()))
}

pub(crate) fn hovered(rect: Rect) -> bool {
    hover_pos().is_some_and(|p| rect.contains(p))
}

/// A dark floating panel.
pub(crate) fn panel(rect: Rect, s: f32) {
    let r = 16.0 * s;
    soft_shadow(
        Rect::new(rect.x, rect.y + 6.0 * s, rect.w, rect.h),
        r,
        18.0 * s,
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
    rounded_rect_gradient(rect, r, rgb(0x231e2e), PANEL);
    rounded_rect_outline(rect, r, 2.0 * s, PANEL_EDGE);
    rounded_rect_outline(
        Rect::new(
            rect.x + 3.0 * s,
            rect.y + 3.0 * s,
            rect.w - 6.0 * s,
            rect.h - 6.0 * s,
        ),
        r - 3.0 * s,
        1.0 * s,
        Color::new(1.0, 1.0, 1.0, 0.05),
    );
}

/// A lighter inset area within a panel.
pub(crate) fn card(rect: Rect, s: f32) {
    let r = 11.0 * s;
    rounded_rect(rect, r, faded(PANEL_LIGHT, 0.8));
    rounded_rect_outline(rect, r, 1.0 * s, Color::new(1.0, 1.0, 1.0, 0.06));
}

/// A small all-caps heading.
pub(crate) fn heading(sprites: &Sprites, label: &str, pos: Vec2, s: f32) {
    text::draw_aligned(
        sprites,
        label,
        pos,
        Align::Left,
        Face::Display,
        15.0 * s,
        TEXT_DIM,
    );
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ButtonKind {
    Normal,
    /// The main call to action, in gold.
    Primary,
}

/// A button: icon, label and the key or button that triggers it.
pub(crate) struct Button<'a> {
    pub(crate) rect: Rect,
    pub(crate) icon: Option<SpriteId>,
    pub(crate) label: &'a str,
    pub(crate) glyph: Option<Glyph>,
    pub(crate) kind: ButtonKind,
    pub(crate) enabled: bool,
    /// Keyboard/controller focus (menus).
    pub(crate) focused: bool,
}

impl Button<'_> {
    pub(crate) fn draw(&self, sprites: &Sprites, s: f32) {
        let r = self.rect;
        let hot = self.enabled && (self.focused || hovered(r));
        let lift = if hot { 2.0 * s } else { 0.0 };
        let face = Rect::new(r.x, r.y - lift, r.w, r.h);
        let radius = (r.h * 0.28).min(14.0 * s);
        let (top, bottom, label_color) = match (self.kind, self.enabled) {
            (_, false) => (rgb(0x2a2533), rgb(0x221e2a), faded(TEXT_DIM, 0.5)),
            (ButtonKind::Primary, true) => (rgb(0xffd978), rgb(0xe0a63a), INK),
            (ButtonKind::Normal, true) if hot => (rgb(0x4a4160), rgb(0x3a3249), TEXT),
            (ButtonKind::Normal, true) => (rgb(0x3a3249), rgb(0x2c2638), TEXT),
        };
        rounded_rect(
            Rect::new(r.x, r.y + 3.0 * s, r.w, r.h),
            radius,
            Color::new(0.0, 0.0, 0.0, 0.4),
        );
        rounded_rect_gradient(face, radius, top, bottom);
        let edge = if self.focused && self.enabled {
            GOLD
        } else {
            faded(INK, 0.9)
        };
        rounded_rect_outline(
            face,
            radius,
            if self.focused { 2.5 * s } else { 1.5 * s },
            edge,
        );
        rounded_rect_outline(
            Rect::new(
                face.x + 2.0 * s,
                face.y + 2.0 * s,
                face.w - 4.0 * s,
                face.h - 4.0 * s,
            ),
            radius - 2.0 * s,
            1.0 * s,
            Color::new(
                1.0,
                1.0,
                1.0,
                if self.kind == ButtonKind::Primary {
                    0.35
                } else {
                    0.07
                },
            ),
        );

        let pad = (r.h * 0.3).min(14.0 * s);
        let cy = face.center().y;
        let icon_size = r.h * 0.5;
        let glyph_h = (r.h * 0.5).min(26.0 * s);
        let glyph_w = self
            .glyph
            .map_or(0.0, |g| glyphs::width(sprites, g, glyph_h));
        let font = (r.h * 0.36).min(22.0 * s);
        let label_w = text::width(sprites, self.label, Face::Display, font);
        let icon_w = if self.icon.is_some() {
            icon_size + 6.0 * s
        } else {
            0.0
        };
        let content = icon_w + label_w;
        let mut x = if self.glyph.is_some() {
            face.x + pad
        } else {
            face.center().x - content / 2.0
        };
        if let Some(icon) = self.icon {
            sprites.draw_at(icon, vec2(x + icon_size / 2.0, cy), icon_size, label_color);
            x += icon_w;
        }
        if !self.label.is_empty() {
            text::draw_aligned(
                sprites,
                self.label,
                vec2(x, cy),
                Align::Left,
                Face::Display,
                font,
                label_color,
            );
        }
        if let Some(glyph) = self.glyph {
            glyphs::draw(
                sprites,
                glyph,
                vec2(face.right() - pad - glyph_w, cy),
                glyph_h,
            );
        }
    }

    /// Whether a tap at `pos` presses this button.
    pub(crate) fn hit(&self, pos: Vec2) -> bool {
        self.enabled && self.rect.contains(pos)
    }
}

/// A horizontal slider showing `value` in [0, 1] across `rect`.
pub(crate) fn slider(rect: Rect, value: f32, focused: bool, s: f32) {
    let h = rect.h * 0.28;
    let track = Rect::new(rect.x, rect.center().y - h / 2.0, rect.w, h);
    rounded_rect(track, h / 2.0, rgb(0x15121c));
    rounded_rect(
        Rect::new(track.x, track.y, (track.w * value).max(h), track.h),
        h / 2.0,
        if focused { GOLD } else { rgb(0xb59a5a) },
    );
    rounded_rect_outline(track, h / 2.0, 1.0 * s, INK);
    let knob = vec2(track.x + track.w * value, track.center().y);
    crate::render::shapes::circle(knob, rect.h * 0.36, INK);
    crate::render::shapes::circle(
        knob,
        rect.h * 0.3,
        if focused { rgb(0xfff2c8) } else { TEXT },
    );
}
