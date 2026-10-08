//! Turning game events into effects and sounds.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use super::LevelScene;
use crate::atlas::SpriteId;
use crate::audio::Sfx;
use crate::direction::Dir4;
use crate::game::GameEvent;
use crate::grid::{Cell, Player};
use crate::position::Position;
use crate::render::board::{self, BoardLayout, CREATURE_SPAN, facing_angle};
use crate::render::fx::{Blend, Ease, Keyframe, Particle, Tween, cell_center};
use crate::render::palette::{self, GOLD, rgb};
use crate::render::shapes::faded;
use crate::render::ui::ScreenLayout;
use crate::scenes::Ctx;

impl LevelScene {
    pub(super) fn confetti(&mut self, layout: &ScreenLayout) {
        let board = self.board_layout(layout);
        let w = board.width as f32;
        let colors = [
            GOLD,
            palette::PLAYER1,
            palette::PLAYER2,
            rgb(0x7bd389),
            rgb(0xff6bd6),
        ];
        for i in 0..90 {
            let mut p = Particle::new(
                SpriteId::Confetti,
                vec2(gen_range(0.0, w), -1.0 - gen_range(0.0, 2.0)),
                gen_range(1.6, 2.8),
            );
            p.vel = vec2(gen_range(-1.5, 1.5), gen_range(1.0, 4.0));
            p.accel = vec2(0.0, 3.0);
            p.drag = 1.2;
            p.spin = gen_range(-9.0, 9.0);
            p.size = (0.22, 0.18);
            let color = colors[i % colors.len()];
            p.color = (color, faded(color, 0.0));
            self.fx.spawn(p);
        }
    }
    /// Ambient particles: dust motes drifting in the light.
    pub(super) fn ambient(&mut self, board: BoardLayout) {
        if gen_range(0.0, 1.0) < 0.08 {
            let (w, h) = (board.width as f32, board.height as f32);
            let mut p = Particle::new(
                SpriteId::Glow,
                vec2(gen_range(0.0, w), gen_range(0.0, h)),
                gen_range(3.0, 6.0),
            );
            p.vel = vec2(gen_range(-0.08, 0.08), gen_range(-0.12, -0.02));
            p.size = (0.06, 0.1);
            let c = self.palette.motes;
            p.color = (faded(c, 0.0), faded(c, 0.0));
            p.color.0.a = 0.35;
            p.blend = Blend::Additive;
            self.fx.spawn(p);
        }
    }
    /// Turn what happened in the game into effects and sounds.
    pub(super) fn react(&mut self, ctx: &mut Ctx, events: Vec<GameEvent>) {
        let mut moves = Vec::new();
        for event in events {
            match event {
                GameEvent::Turn => {
                    self.facings.begin_moves(&moves);
                    moves.clear();
                }
                GameEvent::Moved { entity, from, to } => {
                    moves.push((entity, from, to));
                    self.on_moved(ctx, entity, from, to);
                }
                GameEvent::Arrived {
                    pos,
                    entity,
                    displaced,
                } => self.on_arrived(ctx, pos, entity, displaced),
                GameEvent::Swallowed { pos, entity } => {
                    self.on_swallowed(ctx, pos, entity);
                }
                GameEvent::Contested { pos, cleared } => self.on_destroyed(ctx, pos, cleared, None),
                GameEvent::Exploded { pos, center } => self.on_exploded(ctx, pos, center),
                GameEvent::Blasted { pos, cell } => self.on_destroyed(ctx, pos, cell, None),
                GameEvent::Zapped { pos, digit } => self.on_zapped(ctx, pos, digit),
                GameEvent::WallRaised { pos } => {
                    self.fx.raise_wall(pos);
                    self.dust_ring(cell_center(pos), 6, 0.45);
                    ctx.audio.play_at(Sfx::WallRise, 0.8);
                }
            }
        }
        self.facings.begin_moves(&moves);
    }
    pub(super) fn on_moved(&mut self, ctx: &mut Ctx, entity: Cell, from: Position, to: Position) {
        match entity {
            Cell::Player(_, dir) if from == to => {
                let d = board::dir_vector(dir);
                self.fx.nudge(to, d);
                self.dust_ring(cell_center(to) + d * 0.45, 4, 0.25);
                ctx.audio.play(Sfx::Bump);
            }
            Cell::Player(..) => {
                self.puff(cell_center(from) + vec2(0.0, 0.25), 0.22);
                ctx.audio.play(Sfx::Step);
            }
            Cell::Rat(_) if from != to => ctx.audio.play_at(Sfx::Scurry, 0.8),
            Cell::CyborgRat(_) if from != to => ctx.audio.play_at(Sfx::Servo, 0.8),
            _ => {}
        }
    }
    pub(super) fn on_arrived(
        &mut self,
        ctx: &mut Ctx,
        pos: Position,
        entity: Cell,
        displaced: Cell,
    ) {
        let at = cell_center(pos);
        match (entity, displaced) {
            (Cell::Player(_, dir), Cell::Rat(_) | Cell::CyborgRat(_)) => {
                self.slash(at, dir);
                self.creature_dies(displaced, at, board::dir_vector(dir));
                ctx.audio.play(Sfx::Slash);
                ctx.audio.play(Sfx::Hit);
                ctx.audio.play(match displaced {
                    Cell::Rat(_) => Sfx::Squeak,
                    _ => Sfx::CyborgDeath,
                });
                self.fx.hitstop(0.05);
                self.fx.shake(0.18);
            }
            (Cell::Rat(_) | Cell::CyborgRat(_), Cell::Player(..))
            | (Cell::Player(..), Cell::Player(..)) => {
                self.hero_dies(ctx, displaced, at);
            }
            (Cell::CyborgRat(_), Cell::Rat(_)) => {
                self.creature_dies(displaced, at, Vec2::ZERO);
                self.sparks(at, rgb(0x8ef6ff), 10);
                ctx.audio.play(Sfx::Chomp);
            }
            (_, Cell::Trigger(digit)) => {
                let color = palette::trigger(digit);
                self.fx.flash(at, 1.2, color, 0.35);
                self.fx.ring(at, (0.2, 0.9), color, 0.4);
                self.stars(at, color, 8);
                ctx.audio.play(Sfx::Trigger);
            }
            (_, Cell::Plank | Cell::Spiderweb) => self.on_destroyed(ctx, pos, displaced, None),
            _ => {}
        }
    }
    pub(super) fn on_destroyed(
        &mut self,
        ctx: &mut Ctx,
        pos: Position,
        cell: Cell,
        push: Option<Vec2>,
    ) {
        let at = cell_center(pos);
        match cell {
            Cell::Plank => {
                for _ in 0..10 {
                    let mut p = Particle::new(SpriteId::Splinter, at, gen_range(0.5, 0.9));
                    p.vel = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(1.5, 4.0);
                    p.accel = vec2(0.0, 6.0);
                    p.drag = 2.0;
                    p.spin = gen_range(-14.0, 14.0);
                    p.size = (0.32, 0.26);
                    p.color = (WHITE, faded(WHITE, 0.0));
                    self.fx.spawn(p);
                }
                self.puff(at, 0.5);
                self.fx.shake(0.12);
                ctx.audio.play(Sfx::PlankBreak);
            }
            Cell::Spiderweb => {
                for _ in 0..12 {
                    let mut p = Particle::new(SpriteId::Strand, at, gen_range(0.8, 1.3));
                    p.vel = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.8, 2.2);
                    p.accel = vec2(0.0, 0.8);
                    p.drag = 2.5;
                    p.spin = gen_range(-5.0, 5.0);
                    p.size = (0.55, 0.4);
                    p.color = (WHITE, faded(WHITE, 0.0));
                    self.fx.spawn(p);
                }
                ctx.audio.play(Sfx::WebTear);
            }
            Cell::Rat(_) | Cell::CyborgRat(_) => {
                self.creature_dies(cell, at, push.unwrap_or_default());
                ctx.audio.play(match cell {
                    Cell::Rat(_) => Sfx::Squeak,
                    _ => Sfx::CyborgDeath,
                });
            }
            Cell::Player(..) => self.hero_dies(ctx, cell, at),
            Cell::Trigger(digit) => {
                self.fx.flash(at, 1.0, palette::trigger(digit), 0.3);
                ctx.audio.play(Sfx::Trigger);
            }
            Cell::Empty | Cell::Wall | Cell::BlackHole | Cell::Explosive => {}
        }
    }
    pub(super) fn on_swallowed(&mut self, ctx: &mut Ctx, pos: Position, entity: Cell) {
        let at = cell_center(pos);
        let angle = facing_angle(entity).unwrap_or(0.0);
        self.fx.tween(
            Tween::new(
                creature_sprite(entity),
                Keyframe {
                    pos: at,
                    scale: CREATURE_SPAN,
                    rotation: angle,
                    color: WHITE,
                },
                Keyframe {
                    pos: at,
                    scale: 0.0,
                    rotation: angle + TAU * 1.5,
                    color: Color::new(0.6, 0.3, 1.0, 0.0),
                },
                0.5,
            )
            .ease(Ease::In),
        );
        for _ in 0..14 {
            let a = gen_range(0.0, TAU);
            let mut p = Particle::new(SpriteId::Glow, at + Vec2::from_angle(a) * 0.7, 0.5);
            p.vel = Vec2::from_angle(a + PI * 0.6) * 1.4;
            p.drag = 1.0;
            p.size = (0.16, 0.0);
            p.color = (
                Color::new(0.85, 0.5, 1.0, 0.9),
                Color::new(0.5, 0.2, 1.0, 0.0),
            );
            p.blend = Blend::Additive;
            self.fx.spawn(p);
        }
        if matches!(entity, Cell::Player(..)) {
            self.fx.wash(Color::new(0.4, 0.1, 0.7, 0.35), 0.6);
            ctx.audio.play(Sfx::HeroDeath);
        }
        ctx.audio.play(Sfx::Swallow);
    }
    pub(super) fn on_exploded(&mut self, ctx: &mut Ctx, pos: Position, center: Cell) {
        let at = cell_center(pos);
        self.fx
            .flash(at, 2.4, Color::new(1.0, 0.85, 0.5, 1.0), 0.35);
        self.fx
            .ring(at, (0.3, 1.8), Color::new(1.0, 0.8, 0.5, 0.8), 0.45);
        // Fireball: hot puffs that cool to smoke.
        for _ in 0..22 {
            let off = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.0, 1.1);
            let mut p = Particle::new(SpriteId::Puff, at + off * 0.4, gen_range(0.5, 0.9));
            p.vel = off * gen_range(1.5, 3.0);
            p.accel = vec2(0.0, -1.2);
            p.drag = 3.0;
            p.spin = gen_range(-2.0, 2.0);
            p.size = (gen_range(0.6, 0.9), gen_range(1.0, 1.4));
            p.color = (rgb(0xff9a3c), Color::new(0.16, 0.14, 0.15, 0.0));
            self.fx.spawn(p);
        }
        for _ in 0..10 {
            let off = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.3, 1.2);
            let mut p = Particle::new(SpriteId::Puff, at + off, gen_range(1.0, 1.6));
            p.vel = off * 0.6 + vec2(0.0, -0.4);
            p.drag = 1.0;
            p.size = (0.8, 1.6);
            p.color = (
                Color::new(0.22, 0.2, 0.22, 0.75),
                Color::new(0.12, 0.11, 0.12, 0.0),
            );
            p.spin = gen_range(-1.0, 1.0);
            self.fx.spawn(p);
        }
        for _ in 0..14 {
            let off = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.0, 0.9);
            let mut p = Particle::new(SpriteId::Glow, at + off * 0.5, gen_range(0.25, 0.5));
            p.vel = off * gen_range(2.0, 3.5);
            p.drag = 4.0;
            p.size = (gen_range(0.9, 1.3), 0.3);
            p.color = (rgb(0xffb04a), Color::new(0.9, 0.2, 0.05, 0.0));
            p.blend = Blend::Additive;
            self.fx.spawn(p);
        }
        self.sparks(at, rgb(0xffb347), 18);
        let mut scorch = Particle::new(SpriteId::Shadow, at, 2.5);
        scorch.size = (2.2, 2.4);
        scorch.color = (Color::new(1.0, 1.0, 1.0, 0.75), faded(WHITE, 0.0));
        scorch.under = true;
        self.fx.spawn(scorch);
        self.fx.shake(0.55);
        self.fx.hitstop(0.05);
        ctx.audio.play(Sfx::Explosion);
        if center != Cell::Explosive {
            self.on_destroyed(ctx, pos, center, None);
        }
    }
    pub(super) fn on_zapped(&mut self, ctx: &mut Ctx, pos: Position, digit: u8) {
        let at = cell_center(pos);
        let color = palette::trigger(digit);
        self.fx.raise_wall(pos);
        self.fx.flash(at, 1.6, color, 0.4);
        for dir in crate::direction::Dir8::all() {
            let d = dir.delta();
            let to = at + vec2(d.dx as f32, d.dy as f32);
            self.fx.bolt(at, to, color, gen_range(0.25, 0.4));
        }
        self.sparks(at, color, 10);
        self.fx.shake(0.22);
        ctx.audio.play(Sfx::Zap);
    }
    pub(super) fn hero_dies(&mut self, ctx: &mut Ctx, hero: Cell, at: Vec2) {
        let angle = facing_angle(hero).unwrap_or(0.0);
        self.fx.tween(
            Tween::new(
                creature_sprite(hero),
                Keyframe {
                    pos: at,
                    scale: CREATURE_SPAN,
                    rotation: angle,
                    color: Color::new(1.0, 0.4, 0.4, 1.0),
                },
                Keyframe {
                    pos: at + vec2(0.0, 0.3),
                    scale: CREATURE_SPAN * 0.7,
                    rotation: angle + PI * 1.5,
                    color: Color::new(0.6, 0.1, 0.1, 0.0),
                },
                0.9,
            )
            .arc(0.5),
        );
        self.stars(at, WHITE, 8);
        self.fx.wash(Color::new(0.9, 0.1, 0.15, 0.26), 0.5);
        self.fx.shake(0.6);
        self.fx.hitstop(0.08);
        ctx.audio.play(Sfx::HeroDeath);
    }
    /// A creature flips and tumbles away as it's defeated.
    pub(super) fn creature_dies(&mut self, creature: Cell, at: Vec2, push: Vec2) {
        let angle = facing_angle(creature).unwrap_or(0.0);
        let spin = if gen_range(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
        let fly = push * 0.6 + vec2(gen_range(-0.3, 0.3), 0.1);
        self.fx.tween(
            Tween::new(
                creature_sprite(creature),
                Keyframe {
                    pos: at,
                    scale: CREATURE_SPAN,
                    rotation: angle,
                    color: WHITE,
                },
                Keyframe {
                    pos: at + fly,
                    scale: CREATURE_SPAN * 0.55,
                    rotation: angle + spin * TAU * 0.9,
                    color: Color::new(1.0, 1.0, 1.0, 0.0),
                },
                0.5,
            )
            .arc(0.55),
        );
        let fur = match creature {
            Cell::CyborgRat(_) => rgb(0x9fb0c8),
            _ => rgb(0xa8927f),
        };
        for _ in 0..9 {
            let mut p = Particle::new(
                if matches!(creature, Cell::CyborgRat(_)) {
                    SpriteId::Shard
                } else {
                    SpriteId::Tuft
                },
                at,
                gen_range(0.5, 0.9),
            );
            p.vel = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(1.0, 3.2) + push;
            p.accel = vec2(0.0, 3.0);
            p.drag = 2.5;
            p.spin = gen_range(-10.0, 10.0);
            p.size = (0.28, 0.18);
            p.color = (fur, faded(fur, 0.0));
            self.fx.spawn(p);
        }
        self.stars(at, rgb(0xfff3c4), 6);
        if matches!(creature, Cell::CyborgRat(_)) {
            self.sparks(at, rgb(0x8ef6ff), 12);
        }
    }
    /// A sword swoosh at `at`, swung toward the facing.
    pub(super) fn slash(&mut self, at: Vec2, dir: Dir4) {
        let angle = facing_angle(Cell::Player(Player::Player1, dir)).unwrap_or(0.0);
        let back = -board::dir_vector(dir) * 0.35;
        self.fx.tween(
            Tween::new(
                SpriteId::Slash,
                Keyframe {
                    pos: at + back,
                    scale: 1.2,
                    rotation: angle - 0.5,
                    color: WHITE,
                },
                Keyframe {
                    pos: at + back * 0.4,
                    scale: 1.7,
                    rotation: angle + 0.4,
                    color: faded(WHITE, 0.0),
                },
                0.22,
            )
            .ease(Ease::Out),
        );
        self.fx
            .flash(at, 0.7, Color::new(1.0, 1.0, 0.9, 0.55), 0.15);
    }
    pub(super) fn sparks(&mut self, at: Vec2, color: Color, count: usize) {
        for _ in 0..count {
            let mut p = Particle::new(SpriteId::Spark, at, gen_range(0.25, 0.55));
            p.vel = Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(3.0, 7.0);
            p.accel = vec2(0.0, 5.0);
            p.drag = 3.0;
            p.size = (0.18, 0.05);
            p.stretch = 3.0;
            p.color = (color, faded(color, 0.0));
            p.blend = Blend::Additive;
            self.fx.spawn(p);
        }
    }
    pub(super) fn stars(&mut self, at: Vec2, color: Color, count: usize) {
        for i in 0..count {
            let a = i as f32 / count as f32 * TAU + gen_range(-0.3, 0.3);
            let mut p = Particle::new(SpriteId::Star, at, gen_range(0.35, 0.55));
            p.vel = Vec2::from_angle(a) * gen_range(1.8, 3.0);
            p.drag = 4.0;
            p.spin = gen_range(-4.0, 4.0);
            p.size = (0.32, 0.05);
            p.color = (color, faded(color, 0.0));
            p.blend = Blend::Additive;
            self.fx.spawn(p);
        }
    }
    pub(super) fn puff(&mut self, at: Vec2, size: f32) {
        for _ in 0..3 {
            let mut p = Particle::new(SpriteId::Puff, at, gen_range(0.35, 0.55));
            p.vel = vec2(gen_range(-0.5, 0.5), gen_range(-0.4, 0.0));
            p.drag = 3.0;
            p.size = (size * 0.5, size);
            p.color = (
                Color::new(0.85, 0.82, 0.78, 0.45),
                Color::new(0.85, 0.82, 0.78, 0.0),
            );
            p.under = true;
            self.fx.spawn(p);
        }
    }
    pub(super) fn dust_ring(&mut self, at: Vec2, count: usize, size: f32) {
        for i in 0..count {
            let a = i as f32 / count as f32 * TAU;
            let mut p = Particle::new(SpriteId::Puff, at, gen_range(0.4, 0.6));
            p.vel = Vec2::from_angle(a) * 1.2;
            p.drag = 4.0;
            p.size = (size * 0.6, size);
            p.color = (
                Color::new(0.8, 0.77, 0.74, 0.5),
                Color::new(0.8, 0.77, 0.74, 0.0),
            );
            self.fx.spawn(p);
        }
    }
}

fn creature_sprite(cell: Cell) -> SpriteId {
    match cell {
        Cell::Player(Player::Player1, _) => SpriteId::Hero1,
        Cell::Player(Player::Player2, _) => SpriteId::Hero2,
        Cell::CyborgRat(_) => SpriteId::Cyborg0,
        _ => SpriteId::Rat0,
    }
}
