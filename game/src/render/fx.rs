//! Cosmetic effects layered over the board: particles, keyframed sprites
//! (dying creatures), lightning, flashes, rising walls, bumps, screen shake
//! and hit-stop. Positions are in board cells so effects scale with the
//! board.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::atlas::SpriteId;
use crate::position::Position;
use crate::render::shapes::{faded, lerp_color, polyline};
use crate::sprites::Sprites;

/// Maps board cells to screen pixels.
#[derive(Clone, Copy)]
pub(crate) struct BoardSpace {
    /// Screen position of the top-left corner of cell (0, 0).
    pub(crate) origin: Vec2,
    /// Cell size in pixels.
    pub(crate) cell: f32,
}

impl BoardSpace {
    /// Screen position of a point in cell coordinates.
    pub(crate) fn to_screen(self, p: Vec2) -> Vec2 {
        self.origin + p * self.cell
    }

    pub(crate) fn cell_center(self, pos: Position) -> Vec2 {
        self.to_screen(cell_center(pos))
    }
}

/// Center of a cell in cell coordinates.
pub(crate) fn cell_center(pos: Position) -> Vec2 {
    vec2(pos.x as f32 + 0.5, pos.y as f32 + 0.5)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Blend {
    Alpha,
    Additive,
}

#[derive(Clone, Copy)]
pub(crate) struct Particle {
    pub(crate) sprite: SpriteId,
    pub(crate) pos: Vec2,
    pub(crate) vel: Vec2,
    /// Constant acceleration (e.g. gravity, buoyancy), cells/s².
    pub(crate) accel: Vec2,
    /// Fraction of velocity lost per second.
    pub(crate) drag: f32,
    pub(crate) life: f32,
    /// Size in cells at birth and death.
    pub(crate) size: (f32, f32),
    pub(crate) color: (Color, Color),
    pub(crate) rotation: f32,
    pub(crate) spin: f32,
    /// Stretch along the velocity (sparks), 1 = none.
    pub(crate) stretch: f32,
    pub(crate) blend: Blend,
    /// Drawn on the floor beneath creatures (dust, scorch) rather than over.
    pub(crate) under: bool,
    age: f32,
}

impl Particle {
    pub(crate) fn new(sprite: SpriteId, pos: Vec2, life: f32) -> Self {
        Self {
            sprite,
            pos,
            vel: Vec2::ZERO,
            accel: Vec2::ZERO,
            drag: 0.0,
            life,
            size: (0.2, 0.2),
            color: (WHITE, WHITE),
            rotation: gen_range(0.0, TAU),
            spin: 0.0,
            stretch: 1.0,
            blend: Blend::Alpha,
            under: false,
            age: 0.0,
        }
    }
}

/// A sprite animated between two keyframes, e.g. a creature being flung
/// away as it dies.
#[derive(Clone, Copy)]
pub(crate) struct Tween {
    pub(crate) sprite: SpriteId,
    pub(crate) from: Keyframe,
    pub(crate) to: Keyframe,
    pub(crate) life: f32,
    /// Height of a ballistic hop over the tween's life, in cells.
    arc: f32,
    ease: Ease,
    age: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct Keyframe {
    pub(crate) pos: Vec2,
    pub(crate) scale: f32,
    pub(crate) rotation: f32,
    pub(crate) color: Color,
}

#[derive(Clone, Copy)]
pub(crate) enum Ease {
    Out,
    In,
}

impl Ease {
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Out => 1.0 - (1.0 - t).powi(3),
            Ease::In => t * t * t,
        }
    }
}

impl Tween {
    pub(crate) fn new(sprite: SpriteId, from: Keyframe, to: Keyframe, life: f32) -> Self {
        Self {
            sprite,
            from,
            to,
            life,
            arc: 0.0,
            ease: Ease::Out,
            age: 0.0,
        }
    }

    /// Hop `height` cells up and back down over the tween.
    pub(crate) fn arc(self, height: f32) -> Self {
        Self {
            arc: height,
            ..self
        }
    }

    pub(crate) fn ease(self, ease: Ease) -> Self {
        Self { ease, ..self }
    }
}

struct Bolt {
    from: Vec2,
    to: Vec2,
    color: Color,
    life: f32,
    age: f32,
    seed: u32,
}

struct Flash {
    pos: Vec2,
    radius: f32,
    color: Color,
    life: f32,
    age: f32,
}

struct Ring {
    pos: Vec2,
    radius: (f32, f32),
    color: Color,
    life: f32,
    age: f32,
}

/// A brief shove of the entity at a cell, e.g. bumping into a wall.
struct Nudge {
    pos: Position,
    dir: Vec2,
    age: f32,
}

const NUDGE_TIME: f32 = 0.16;
const RISE_TIME: f32 = 0.32;

#[derive(Default)]
pub(crate) struct Fx {
    particles: Vec<Particle>,
    tweens: Vec<Tween>,
    bolts: Vec<Bolt>,
    flashes: Vec<Flash>,
    rings: Vec<Ring>,
    nudges: Vec<Nudge>,
    /// Walls growing out of the floor, by age.
    rising: Vec<(Position, f32)>,
    /// Screen shake intensity in [0, 1]; decays over time.
    trauma: f32,
    /// Remaining freeze time for impact.
    hitstop: f32,
    /// A full-board color wash, e.g. red on death.
    wash: Option<Flash>,
    time: f32,
}

impl Fx {
    pub(crate) fn update(&mut self, dt: f32) {
        self.time += dt;
        self.hitstop = (self.hitstop - dt).max(0.0);
        self.trauma = (self.trauma - dt * 1.6).max(0.0);
        for p in &mut self.particles {
            p.age += dt;
            p.vel += p.accel * dt;
            p.vel *= (1.0 - p.drag * dt).max(0.0);
            p.pos += p.vel * dt;
            p.rotation += p.spin * dt;
        }
        self.particles.retain(|p| p.age < p.life);
        for t in &mut self.tweens {
            t.age += dt;
        }
        self.tweens.retain(|t| t.age < t.life);
        for b in &mut self.bolts {
            b.age += dt;
        }
        self.bolts.retain(|b| b.age < b.life);
        for f in &mut self.flashes {
            f.age += dt;
        }
        self.flashes.retain(|f| f.age < f.life);
        for r in &mut self.rings {
            r.age += dt;
        }
        self.rings.retain(|r| r.age < r.life);
        for n in &mut self.nudges {
            n.age += dt;
        }
        self.nudges.retain(|n| n.age < NUDGE_TIME);
        for (_, age) in &mut self.rising {
            *age += dt;
        }
        self.rising.retain(|&(_, age)| age < RISE_TIME);
        if let Some(wash) = &mut self.wash {
            wash.age += dt;
            if wash.age >= wash.life {
                self.wash = None;
            }
        }
    }

    /// Drop every effect (e.g. after undo, when they'd no longer match the board).
    pub(crate) fn clear(&mut self) {
        let time = self.time;
        *self = Self {
            time,
            ..Self::default()
        };
    }

    pub(crate) fn spawn(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    pub(crate) fn tween(&mut self, tween: Tween) {
        self.tweens.push(tween);
    }

    pub(crate) fn bolt(&mut self, from: Vec2, to: Vec2, color: Color, life: f32) {
        self.bolts.push(Bolt {
            from,
            to,
            color,
            life,
            age: 0.0,
            seed: gen_range(0, u32::MAX),
        });
    }

    pub(crate) fn flash(&mut self, pos: Vec2, radius: f32, color: Color, life: f32) {
        self.flashes.push(Flash {
            pos,
            radius,
            color,
            life,
            age: 0.0,
        });
    }

    pub(crate) fn ring(&mut self, pos: Vec2, radius: (f32, f32), color: Color, life: f32) {
        self.rings.push(Ring {
            pos,
            radius,
            color,
            life,
            age: 0.0,
        });
    }

    pub(crate) fn nudge(&mut self, pos: Position, dir: Vec2) {
        self.nudges.push(Nudge { pos, dir, age: 0.0 });
    }

    pub(crate) fn raise_wall(&mut self, pos: Position) {
        self.rising.push((pos, 0.0));
    }

    pub(crate) fn shake(&mut self, amount: f32) {
        self.shake_up_to(amount, 1.0);
    }

    /// Shake, but not past `limit` (nor lessening a bigger shake already
    /// under way).
    pub(crate) fn shake_up_to(&mut self, amount: f32, limit: f32) {
        self.trauma = (self.trauma + amount).min(limit.max(self.trauma));
    }

    pub(crate) fn hitstop(&mut self, duration: f32) {
        self.hitstop = self.hitstop.max(duration);
    }

    /// Whether gameplay animation should be frozen for impact.
    pub(crate) fn frozen(&self) -> bool {
        self.hitstop > 0.0
    }

    pub(crate) fn wash(&mut self, color: Color, life: f32) {
        self.wash = Some(Flash {
            pos: Vec2::ZERO,
            radius: 0.0,
            color,
            life,
            age: 0.0,
        });
    }

    /// Pixel offset to shake the board by.
    pub(crate) fn shake_offset(&self, cell: f32) -> Vec2 {
        let k = self.trauma * self.trauma * cell * 0.22;
        let t = self.time * 38.0;
        vec2(
            (t.sin() + (t * 1.7 + 1.3).sin() * 0.5) * k,
            ((t * 1.3 + 2.1).sin() + (t * 2.3).sin() * 0.5) * k,
        )
    }

    /// Offset (in cells) of the entity at `pos` due to bumps.
    pub(crate) fn nudge_offset(&self, pos: Position) -> Vec2 {
        self.nudges
            .iter()
            .filter(|n| n.pos == pos)
            .map(|n| n.dir * 0.16 * (PI * n.age / NUDGE_TIME).sin())
            .sum()
    }

    /// How far a recently raised wall at `pos` has grown, in (0, 1], or
    /// None if it's fully up.
    pub(crate) fn rise_progress(&self, pos: Position) -> Option<f32> {
        self.rising
            .iter()
            .find(|&&(p, _)| p == pos)
            .map(|&(_, age)| ease_out_back(age / RISE_TIME))
    }

    /// Draw effects that sit on the board below entities (scorch, dust).
    pub(crate) fn draw_under(&self, sprites: &Sprites, space: BoardSpace) {
        self.draw_particles(sprites, space, Blend::Alpha, true);
    }

    /// Draw effects above everything on the board.
    pub(crate) fn draw_over(&self, sprites: &Sprites, space: BoardSpace) {
        for t in &self.tweens {
            draw_tween(sprites, space, t);
        }
        self.draw_particles(sprites, space, Blend::Alpha, false);
        for r in &self.rings {
            let t = r.age / r.life;
            let radius = r.radius.0 + (r.radius.1 - r.radius.0) * ease_out(t);
            sprites.draw_at(
                SpriteId::Ring,
                space.to_screen(r.pos),
                radius * 2.0 / 0.84 * space.cell,
                faded(r.color, 1.0 - t),
            );
        }
        sprites.additive(|| {
            self.draw_particles(sprites, space, Blend::Additive, false);
            for f in &self.flashes {
                let t = f.age / f.life;
                sprites.draw_at(
                    SpriteId::Glow,
                    space.to_screen(f.pos),
                    f.radius * 2.0 * space.cell * (0.7 + 0.3 * t),
                    faded(f.color, (1.0 - t).powi(2)),
                );
            }
            for b in &self.bolts {
                draw_bolt(space, b, self.time);
            }
        });
    }

    /// Draw the full-board color wash over `rect`.
    pub(crate) fn draw_wash(&self, rect: Rect) {
        if let Some(w) = &self.wash {
            let t = w.age / w.life;
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                faded(w.color, (1.0 - t).powi(2)),
            );
        }
    }

    fn draw_particles(&self, sprites: &Sprites, space: BoardSpace, blend: Blend, under: bool) {
        for p in self
            .particles
            .iter()
            .filter(|p| p.blend == blend && p.under == under)
        {
            let t = p.age / p.life;
            let size = (p.size.0 + (p.size.1 - p.size.0) * t) * space.cell;
            let color = lerp_color(p.color.0, p.color.1, t);
            let (dims, rotation) = if p.stretch > 1.0 {
                let speed = p.vel.length();
                let angle = p.vel.y.atan2(p.vel.x);
                let len = 1.0 + (p.stretch - 1.0) * (speed / 6.0).min(1.0);
                (vec2(size * len, size), angle)
            } else {
                (Vec2::splat(size), p.rotation)
            };
            sprites.draw(p.sprite, space.to_screen(p.pos), dims, rotation, color);
        }
    }
}

fn draw_tween(sprites: &Sprites, space: BoardSpace, t: &Tween) {
    let k = (t.age / t.life).clamp(0.0, 1.0);
    let e = t.ease.apply(k);
    let (a, b) = (t.from, t.to);
    let pos = a.pos + (b.pos - a.pos) * e - vec2(0.0, t.arc * 4.0 * k * (1.0 - k));
    let scale = a.scale + (b.scale - a.scale) * e;
    let rotation = a.rotation + (b.rotation - a.rotation) * e;
    let color = lerp_color(a.color, b.color, e);
    sprites.draw(
        t.sprite,
        space.to_screen(pos),
        Vec2::splat(scale * space.cell),
        rotation,
        color,
    );
}

/// A jagged lightning bolt that re-forks a few times a second.
fn draw_bolt(space: BoardSpace, b: &Bolt, time: f32) {
    let t = b.age / b.life;
    let flicker = (time * 20.0) as u32;
    let mut seed = b.seed ^ flicker.wrapping_mul(2_654_435_761);
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let (a, z) = (space.to_screen(b.from), space.to_screen(b.to));
    let normal = (z - a).perp().normalize_or_zero();
    let segments = 7;
    let jag = space.cell * 0.16;
    let points: Vec<Vec2> = (0..=segments)
        .map(|i| {
            let f = i as f32 / segments as f32;
            let off = if i == 0 || i == segments {
                0.0
            } else {
                rand() * jag
            };
            a + (z - a) * f + normal * off
        })
        .collect();
    let alpha = (1.0 - t).powf(0.7);
    polyline(&points, space.cell * 0.12, faded(b.color, alpha * 0.45));
    polyline(&points, space.cell * 0.04, faded(WHITE, alpha));
}

pub(crate) fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Ease out with a small overshoot past 1.
pub(crate) fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    let s = 1.9;
    1.0 + t * t * ((s + 1.0) * t + s)
}
