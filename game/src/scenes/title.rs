//! The title screen.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::atlas::SpriteId;
use crate::input::InputHints;
use crate::position::Position;
use crate::render::board::SPRITE_SPAN;
use crate::render::glyphs::{self, Glyph, PadButton};
use crate::render::palette::{GOLD, INK, TEXT, TEXT_DIM, palette};
use crate::render::shapes::{faded, radial_gradient};
use crate::render::terrain::cell_hash;
use crate::render::text::{self, Align};
use crate::render::ui::ScreenLayout;
use crate::scenes::{Ctx, FrameInput};
use crate::sprites::{Face, Sprites};
use crate::world_map::Theme;

/// A rat crossing the title screen.
struct Runner {
    pos: Vec2,
    vel: Vec2,
    phase: f32,
}

impl Runner {
    fn spawn() -> Self {
        let (w, h) = (screen_width(), screen_height());
        let from_left = gen_range(0.0, 1.0) < 0.5;
        let y = gen_range(h * 0.55, h * 0.95);
        let speed = gen_range(140.0, 320.0);
        let angle: f32 = gen_range(-0.25, 0.25);
        let dir = if from_left { 1.0 } else { -1.0 };
        Self {
            pos: vec2(if from_left { -80.0 } else { w + 80.0 }, y),
            vel: vec2(dir * speed * angle.cos(), speed * angle.sin()),
            phase: gen_range(0.0, 4.0),
        }
    }
}

pub(crate) struct TitleScene {
    time: f32,
    runners: Vec<Runner>,
    next_runner: f32,
}

impl TitleScene {
    pub(crate) fn new() -> Self {
        Self {
            time: 0.0,
            runners: Vec::new(),
            next_runner: 0.8,
        }
    }

    /// Returns true when the player wants to start.
    pub(crate) fn update(&mut self, input: &FrameInput) -> bool {
        let dt = input.dt;
        self.time += dt;
        self.next_runner -= dt;
        if self.next_runner <= 0.0 {
            self.runners.push(Runner::spawn());
            self.next_runner = gen_range(0.6, 2.2);
        }
        for r in &mut self.runners {
            r.pos += r.vel * dt;
            r.vel = r.vel.rotate(Vec2::from_angle(
                (self.time * 3.0 + r.phase).sin() * 0.6 * dt,
            ));
        }
        let (w, h) = (screen_width(), screen_height());
        self.runners.retain(|r| {
            r.pos.x > -120.0 && r.pos.x < w + 120.0 && r.pos.y > -120.0 && r.pos.y < h + 120.0
        });
        input.any && self.time > 0.4
    }

    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let s = layout.s;
        let (w, h) = (screen_width(), screen_height());
        let cellar = palette(Theme::Cellar);
        clear_background(cellar.backdrop.1);
        draw_floor(sprites, &cellar, self.time, s);
        radial_gradient(
            vec2(w / 2.0, h * 0.42),
            vec2(w, h).length() * 0.6,
            Color::new(0.0, 0.0, 0.0, 0.0),
            Color::new(0.0, 0.0, 0.0, 0.85),
        );

        for r in &self.runners {
            let angle = r.vel.x.atan2(-r.vel.y);
            let size = 74.0 * s;
            sprites.draw(
                SpriteId::Shadow,
                r.pos + vec2(0.0, size * 0.12),
                vec2(size * 0.6, size * 0.4),
                0.0,
                faded(WHITE, 0.7),
            );
            let frame = ((self.time * 16.0 + r.phase * 4.0) as usize) % 4;
            sprites.draw(
                SpriteId::RAT_FRAMES[frame],
                r.pos,
                Vec2::splat(size * SPRITE_SPAN),
                angle,
                WHITE,
            );
        }

        // The big rat lurking behind the title.
        let rat_at = vec2(w / 2.0, h * 0.36);
        let breathe = 1.0 + (self.time * 1.8).sin() * 0.015;
        let rat_size = (h * 0.62).min(w * 0.8) * breathe;
        sprites.additive(|| {
            let eye = 0.6 + 0.4 * (self.time * 2.2).sin().abs();
            sprites.draw_at(
                SpriteId::Glow,
                rat_at,
                rat_size * 1.2,
                Color::new(0.9, 0.15, 0.2, 0.18 * eye),
            )
        });
        sprites.draw_at(SpriteId::TitleRat, rat_at, rat_size, WHITE);

        // Bouncing letters.
        let title = "INFESTATION";
        let size = (w / 9.0).min(h / 6.0).min(150.0 * s);
        let total =
            text::width(sprites, title, Face::Display, size) + size * 0.05 * title.len() as f32;
        let mut x = w / 2.0 - total / 2.0;
        let base_y = h * 0.62;
        let intro = (self.time / 0.9).min(1.0);
        for (i, ch) in title.chars().enumerate() {
            let letter = ch.to_string();
            let lw = text::width(sprites, &letter, Face::Display, size);
            let t = (self.time * 2.4 - i as f32 * 0.32).sin();
            let drop = (1.0 - ((intro * 1.6 - i as f32 * 0.05).clamp(0.0, 1.0))).powi(2) * h * 0.4;
            let center = vec2(x + lw / 2.0, base_y + t * size * 0.04 - drop);
            text::draw_title(sprites, &letter, center, size, GOLD, INK);
            x += lw + size * 0.05;
        }
        text::draw_aligned(
            sprites,
            "Clear the keep of rats, one move at a time.",
            vec2(w / 2.0, base_y + size * 0.62),
            Align::Center,
            Face::Body,
            (size * 0.22).max(18.0 * s),
            faded(TEXT, intro),
        );

        let pulse = 0.55 + 0.45 * (self.time * 3.0).sin();
        let prompt_y = h * 0.86;
        let prompt_size = 26.0 * s;
        let (label, glyph) = match ctx.hints {
            InputHints::Touch => ("Tap to begin", None),
            InputHints::Keyboard => ("Press any key", None),
            InputHints::Controller(ty) => ("to begin", Some(Glyph::Pad(ty, PadButton::South))),
        };
        if self.time > 0.9 {
            let lw = text::width(sprites, label, Face::Display, prompt_size);
            let gh = 36.0 * s;
            let gw = glyph.map_or(0.0, |g| glyphs::width(sprites, g, gh) + 10.0 * s);
            let x0 = w / 2.0 - (lw + gw) / 2.0;
            if let Some(g) = glyph {
                glyphs::draw(sprites, g, vec2(x0, prompt_y), gh);
            }
            text::draw_aligned(
                sprites,
                label,
                vec2(x0 + gw, prompt_y),
                Align::Left,
                Face::Display,
                prompt_size,
                faded(TEXT, pulse),
            );
        }
        text::draw_aligned(
            sprites,
            "Art, music and sound generated by AI",
            vec2(w / 2.0, h - 18.0 * s),
            Align::Center,
            Face::Body,
            14.0 * s,
            faded(TEXT_DIM, 0.6),
        );
    }
}

/// A dim flagstone floor drifting slowly behind the title.
fn draw_floor(sprites: &Sprites, palette: &crate::render::palette::Palette, time: f32, s: f32) {
    let cell = 96.0 * s;
    let drift = vec2(time * 6.0, time * 3.0) % cell;
    let cols = (screen_width() / cell) as i32 + 2;
    let rows = (screen_height() / cell) as i32 + 2;
    let tint = Color::new(
        palette.floor.r * 0.4,
        palette.floor.g * 0.4,
        palette.floor.b * 0.45,
        1.0,
    );
    for y in -1..rows {
        for x in -1..cols {
            let hash = cell_hash(Position { x, y });
            let tile = hash % 16;
            draw_texture_ex(
                &sprites.floor,
                x as f32 * cell - drift.x,
                y as f32 * cell - drift.y,
                tint,
                DrawTextureParams {
                    dest_size: Some(Vec2::splat(cell)),
                    source: Some(Rect::new(
                        (tile % 4) as f32 * 256.0,
                        (tile / 4) as f32 * 256.0,
                        256.0,
                        256.0,
                    )),
                    flip_x: hash & 0x100 != 0,
                    ..Default::default()
                },
            );
        }
    }
}
