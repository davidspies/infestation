//! Drawing a level as a stone diorama: the slab and its walls, floor
//! features, objects, and creatures with movement, turning and idle
//! animation.

use std::collections::HashMap;
use std::f32::consts::PI;

use macroquad::prelude::*;

use crate::atlas::SpriteId;
use crate::direction::{Dir4, Dir8};
use crate::game::Game;
use crate::grid::{Cell, Grid, Player};
use crate::position::{Position, PositionDelta};
use crate::render::fx::{BoardSpace, Fx, cell_center};
use crate::render::palette::{self, INK, Palette};
use crate::render::shapes::{
    circle, faded, rounded_rect_outline, soft_shadow, textured_rounded_rect, vertical_gradient,
};
use crate::render::terrain::{self, Ground, TEXEL_CELL, Terrain, WALL_FRONT, cell_hash, darken};
use crate::render::text;
use crate::sprites::Sprites;

/// Stone frame around the grid, in cells.
const FRAME_SIDE: f32 = 0.42;
const FRAME_TOP: f32 = 0.62;
const FRAME_BOTTOM: f32 = 0.30;
/// Height of the slab's south face below the frame.
const SLAB_FRONT: f32 = 0.42;
/// Sprite canvases span this many cells.
pub(crate) const SPRITE_SPAN: f32 = 256.0 / 192.0;
/// Creatures are drawn larger than their canvas, so they read at a glance.
pub(crate) const CREATURE_SPAN: f32 = SPRITE_SPAN * 1.3;

/// Where the board sits on screen.
#[derive(Clone, Copy)]
pub(crate) struct BoardLayout {
    pub(crate) space: BoardSpace,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl BoardLayout {
    /// The largest board (with its frame) that fits centered in `area`.
    pub(crate) fn fit(area: Rect, width: usize, height: usize) -> Self {
        let span = vec2(
            width as f32 + FRAME_SIDE * 2.0,
            height as f32 + FRAME_TOP + FRAME_BOTTOM + SLAB_FRONT,
        );
        let cell = (area.w / span.x).min(area.h / span.y);
        let top_left = area.center() - span * cell / 2.0;
        Self {
            space: BoardSpace {
                origin: top_left + vec2(FRAME_SIDE, FRAME_TOP) * cell,
                cell,
            },
            width,
            height,
        }
    }

    pub(crate) fn cell_rect(&self, pos: Position) -> Rect {
        let p = self.space.to_screen(vec2(pos.x as f32, pos.y as f32));
        Rect::new(p.x, p.y, self.space.cell, self.space.cell)
    }

    /// The whole slab, frame and south face included.
    pub(crate) fn slab_rect(&self) -> Rect {
        let c = self.space.cell;
        let o = self.space.origin;
        Rect::new(
            o.x - FRAME_SIDE * c,
            o.y - FRAME_TOP * c,
            (self.width as f32 + FRAME_SIDE * 2.0) * c,
            (self.height as f32 + FRAME_TOP + FRAME_BOTTOM + SLAB_FRONT) * c,
        )
    }
}

/// Clockwise screen angle of a facing, with north (the sprites' facing) at 0.
pub(crate) fn facing_angle(cell: Cell) -> Option<f32> {
    let dir8 = match cell {
        Cell::Player(_, dir) => match dir {
            Dir4::North => Dir8::North,
            Dir4::South => Dir8::South,
            Dir4::East => Dir8::East,
            Dir4::West => Dir8::West,
        },
        Cell::Rat(dir) | Cell::CyborgRat(dir) => dir,
        _ => return None,
    };
    let PositionDelta { dx, dy } = dir8.delta();
    Some((dx as f32).atan2(-dy as f32))
}

/// Wrap an angle into (-π, π].
fn wrap_angle(a: f32) -> f32 {
    let a = (a + PI).rem_euclid(2.0 * PI) - PI;
    if a <= -PI { a + 2.0 * PI } else { a }
}

/// Smoothly turning facings of the creatures on the board, keyed by the
/// cell each creature occupies (or is moving to).
#[derive(Default)]
pub(crate) struct Facings {
    angles: HashMap<Position, Facing>,
}

#[derive(Clone, Copy)]
struct Facing {
    current: f32,
    target: f32,
}

impl Facings {
    /// Face every creature as on `grid`, without turning animation.
    pub(crate) fn snap(&mut self, grid: &Grid) {
        self.angles = grid
            .entries()
            .filter_map(|(pos, cell)| {
                let a = facing_angle(cell)?;
                Some((
                    pos,
                    Facing {
                        current: a,
                        target: a,
                    },
                ))
            })
            .collect();
    }

    /// Creatures start moving: carry each one's facing to its destination
    /// and turn it toward its new direction.
    pub(crate) fn begin_moves(&mut self, moves: &[(Cell, Position, Position)]) {
        let taken: Vec<_> = moves
            .iter()
            .map(|&(_, from, _)| self.angles.remove(&from))
            .collect();
        for (&(entity, _, to), facing) in moves.iter().zip(taken) {
            let Some(target) = facing_angle(entity) else {
                continue;
            };
            let current = facing.map_or(target, |f| f.current);
            self.angles.insert(
                to,
                Facing {
                    current,
                    target: current + wrap_angle(target - current),
                },
            );
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        let k = 1.0 - (-dt * 22.0).exp();
        for f in self.angles.values_mut() {
            f.current += (f.target - f.current) * k;
        }
    }

    fn angle(&self, pos: Position, cell: Cell) -> f32 {
        self.angles
            .get(&pos)
            .map(|f| f.current)
            .or_else(|| facing_angle(cell))
            .unwrap_or(0.0)
    }
}

/// Everything needed to draw a level's board this frame.
pub(crate) struct BoardView<'a> {
    pub(crate) game: &'a Game,
    pub(crate) facings: &'a Facings,
    pub(crate) fx: &'a Fx,
    pub(crate) palette: &'a Palette,
    pub(crate) time: f32,
    /// Preregistered moves, drawn translucent at their destination.
    pub(crate) ghosts: &'a [(Position, Cell)],
}

pub(crate) fn draw(sprites: &Sprites, layout: BoardLayout, view: &BoardView) {
    let shake = view.fx.shake_offset(layout.space.cell);
    let layout = BoardLayout {
        space: BoardSpace {
            origin: layout.space.origin + shake,
            ..layout.space
        },
        ..layout
    };
    let grid = view
        .game
        .animation
        .as_ref()
        .map_or(&view.game.state.grid, |handler| &handler.grid);

    let cells: Vec<Position> = grid.entries().map(|(pos, _)| pos).collect();
    let ground = |pos| {
        if grid.at(pos) == Cell::Wall {
            Ground::Wall
        } else {
            Ground::Floor
        }
    };
    let palette = |_| view.palette;
    let rise = |pos| view.fx.rise_progress(pos);
    let terrain = Terrain {
        space: layout.space,
        ground: &ground,
        palette: &palette,
        rise: &rise,
    };

    draw_slab(sprites, layout, view.palette);
    terrain::draw_floor(sprites, &terrain, &cells);
    draw_floor_features(sprites, layout, grid, view);
    terrain::draw_ambient_occlusion(&terrain, &cells);
    view.fx.draw_under(sprites, layout.space);
    draw_frame_front(sprites, layout, grid, view.palette);
    terrain::draw_walls(sprites, &terrain, &cells);
    draw_floor_outlines(layout, grid);
    terrain::draw_outlines(&terrain, &cells);
    draw_objects(sprites, layout, grid);
    draw_creatures(sprites, layout, grid, view);
    view.fx.draw_over(sprites, layout.space);
    view.fx.draw_wash(layout.slab_rect());
}

fn draw_slab(sprites: &Sprites, layout: BoardLayout, palette: &Palette) {
    let c = layout.space.cell;
    let slab = layout.slab_rect();
    soft_shadow(
        Rect::new(slab.x + c * 0.1, slab.y + c * 0.35, slab.w, slab.h),
        c * 0.3,
        c * 0.9,
        Color::new(0.0, 0.0, 0.0, 0.55),
    );
    // South face of the slab.
    let front_h = SLAB_FRONT * c;
    let front = Rect::new(slab.x, slab.bottom() - front_h * 1.6, slab.w, front_h * 1.6);
    let uv = Rect::new(
        -FRAME_SIDE,
        0.0,
        slab.w / c,
        WALL_FRONT / 0.5 * 1.6 * SLAB_FRONT / WALL_FRONT,
    );
    textured_rounded_rect(
        &sprites.wall_front,
        front,
        c * 0.22,
        Rect::new(uv.x / 4.0, uv.y, uv.w / 4.0, uv.h.min(1.0)),
        darken(palette.wall, 0.62),
    );
    // Top surface of the frame (the floor is drawn over its middle).
    let top = Rect::new(slab.x, slab.y, slab.w, slab.h - front_h);
    let o = layout.space.origin;
    let tex = |p: f32| p / c / 4.0;
    textured_rounded_rect(
        &sprites.wall_top,
        top,
        c * 0.22,
        Rect::new(tex(top.x - o.x), tex(top.y - o.y), tex(top.w), tex(top.h)),
        palette.wall,
    );
    // Bevel light along the outer rim and darkening toward the base.
    rounded_rect_outline(
        Rect::new(
            top.x + c * 0.05,
            top.y + c * 0.05,
            top.w - c * 0.1,
            top.h - c * 0.1,
        ),
        c * 0.18,
        c * 0.035,
        Color::new(1.0, 1.0, 1.0, 0.12),
    );
    vertical_gradient(
        Rect::new(front.x, top.bottom(), front.w, slab.bottom() - top.bottom()),
        Color::new(0.0, 0.0, 0.0, 0.0),
        Color::new(0.0, 0.0, 0.0, 0.45),
    );
    rounded_rect_outline(slab, c * 0.22, c * 0.06, INK);
}

/// Things lying on the floor: notes, triggers, black holes and webs.
fn draw_floor_features(sprites: &Sprites, layout: BoardLayout, grid: &Grid, view: &BoardView) {
    let c = layout.space.cell;
    let t = view.time;
    let span = Vec2::splat(c * SPRITE_SPAN);
    // Grouped into passes (glows, then sprites, then digits) so the GPU
    // gets a few large batches rather than several per cell.
    let features: Vec<(Vec2, Cell)> = grid
        .entries()
        .filter(|(_, cell)| matches!(cell, Cell::Trigger(_) | Cell::BlackHole | Cell::Spiderweb))
        .map(|(pos, cell)| (layout.space.cell_center(pos), cell))
        .collect();
    sprites.additive(|| {
        for &(center, cell) in &features {
            match cell {
                Cell::Trigger(digit) => trigger_glow(sprites, center, c, digit, t),
                Cell::BlackHole => sprites.draw_at(
                    SpriteId::Glow,
                    center,
                    c * 1.5 * hole_pulse(center, t),
                    Color::new(0.75, 0.3, 1.0, 0.35),
                ),
                _ => {}
            }
        }
    });
    for (pos, _) in grid.notes() {
        let center = layout.space.cell_center(pos);
        let bob = (t * 2.4 + cell_hash(pos) as f32).sin() * c * 0.03;
        sprites.draw(SpriteId::Note, center + vec2(0.0, bob), span, 0.0, WHITE);
    }
    for &(center, cell) in &features {
        match cell {
            Cell::Trigger(digit) => trigger_plate(sprites, center, c, digit),
            Cell::BlackHole => {
                let pulse = hole_pulse(center, t);
                sprites.draw(SpriteId::HoleBase, center, span, 0.0, WHITE);
                sprites.draw(SpriteId::HoleSwirl, center, span * pulse, t * 1.7, WHITE);
                sprites.draw(
                    SpriteId::HoleSwirl,
                    center,
                    span * 0.62,
                    -t * 2.6 + 1.0,
                    Color::new(1.0, 1.0, 1.0, 0.7),
                );
            }
            Cell::Spiderweb => {
                // Twice, for strands that hold up on light floors.
                sprites.draw(SpriteId::Web, center, span, 0.0, WHITE);
                sprites.draw(
                    SpriteId::Web,
                    center,
                    span,
                    0.0,
                    Color::new(1.0, 1.0, 1.0, 0.6),
                );
            }
            _ => {}
        }
    }
    for &(center, cell) in &features {
        if let Cell::Trigger(digit) = cell {
            trigger_digit(sprites, center, c, digit);
        }
    }
}

fn hole_pulse(center: Vec2, t: f32) -> f32 {
    1.0 + (t * 3.1 + center.x * 0.01).sin() * 0.03
}

/// A trigger plate: glow, plate and digit (for one-off use; the board draws
/// these in passes).
pub(crate) fn draw_trigger(sprites: &Sprites, center: Vec2, c: f32, digit: u8, t: f32) {
    sprites.additive(|| trigger_glow(sprites, center, c, digit, t));
    trigger_plate(sprites, center, c, digit);
    trigger_digit(sprites, center, c, digit);
}

fn trigger_glow(sprites: &Sprites, center: Vec2, c: f32, digit: u8, t: f32) {
    let color = palette::trigger(digit);
    let pulse = 0.5 + 0.5 * (t * 3.0 + digit as f32).sin();
    sprites.draw_at(
        SpriteId::Glow,
        center,
        c * (1.05 + pulse * 0.15),
        faded(color, 0.35 + pulse * 0.15),
    );
}

fn trigger_plate(sprites: &Sprites, center: Vec2, c: f32, digit: u8) {
    let color = palette::trigger(digit);
    sprites.draw(
        SpriteId::TriggerPlate,
        center,
        Vec2::splat(c * SPRITE_SPAN),
        0.0,
        color,
    );
}

fn trigger_digit(sprites: &Sprites, center: Vec2, c: f32, digit: u8) {
    let color = palette::trigger(digit);
    text::draw_title(
        sprites,
        &digit.to_string(),
        center,
        c * 0.42,
        WHITE,
        Color::new(color.r * 0.3, color.g * 0.3, color.b * 0.3, 1.0),
    );
}

/// The frame's south face above floor cells in the first row.
fn draw_frame_front(sprites: &Sprites, layout: BoardLayout, grid: &Grid, palette: &Palette) {
    let c = layout.space.cell;
    for x in 0..layout.width {
        let pos = Position::new(x, 0);
        if grid.at(pos) != Cell::Wall {
            let r = layout.cell_rect(pos);
            terrain::draw_wall_front(
                sprites,
                Rect::new(r.x, r.y - WALL_FRONT * c, r.w, WALL_FRONT * c),
                x as f32,
                darken(palette.wall, 0.9),
            );
        }
    }
}

/// Ink lines where floor meets the frame (which isn't on the grid, so the
/// terrain outlines don't cover it).
fn draw_floor_outlines(layout: BoardLayout, grid: &Grid) {
    let c = layout.space.cell;
    let w = (c * 0.055).max(1.5);
    let bounds = grid.bounds();
    for (pos, cell) in grid.entries() {
        if cell == Cell::Wall {
            continue;
        }
        let r = layout.cell_rect(pos);
        let off_grid = |d: Dir4| !(pos + d.delta()).in_bounds(bounds);
        if off_grid(Dir4::North) {
            draw_rectangle(r.x - w / 2.0, r.y - w / 2.0, r.w + w, w, INK);
        }
        if off_grid(Dir4::South) {
            draw_rectangle(r.x - w / 2.0, r.bottom() - w / 2.0, r.w + w, w, INK);
        }
        if off_grid(Dir4::West) {
            draw_rectangle(r.x - w / 2.0, r.y - w / 2.0, w, r.h + w, INK);
        }
        if off_grid(Dir4::East) {
            draw_rectangle(r.right() - w / 2.0, r.y - w / 2.0, w, r.h + w, INK);
        }
    }
}

/// Planks and kegs: upright objects with a shadow at their base.
fn draw_objects(sprites: &Sprites, layout: BoardLayout, grid: &Grid) {
    let c = layout.space.cell;
    for (pos, cell) in grid.entries() {
        let sprite = match cell {
            Cell::Plank => SpriteId::Plank,
            Cell::Explosive => SpriteId::Keg,
            _ => continue,
        };
        let center = layout.space.cell_center(pos);
        sprites.draw(
            SpriteId::Shadow,
            center + vec2(c * 0.04, c * 0.26),
            vec2(c * 0.95, c * 0.5),
            0.0,
            Color::new(1.0, 1.0, 1.0, 0.75),
        );
        sprites.draw(sprite, center, Vec2::splat(c * SPRITE_SPAN), 0.0, WHITE);
    }
}

/// A creature to draw this frame.
struct Creature {
    cell: Cell,
    /// Board position in cells (center).
    pos: Vec2,
    angle: f32,
    /// Height above the floor in cells (hops).
    lift: f32,
    /// Stretch along / across the facing.
    stretch: Vec2,
    moving: bool,
    phase: f32,
    alpha: f32,
}

fn ease_in_out(t: f32) -> f32 {
    0.5 - 0.5 * (PI * t.clamp(0.0, 1.0)).cos()
}

fn draw_creatures(sprites: &Sprites, layout: BoardLayout, grid: &Grid, view: &BoardView) {
    let mut creatures: Vec<Creature> = grid
        .entries()
        .filter(|&(_, cell)| facing_angle(cell).is_some())
        .map(|(pos, cell)| Creature {
            cell,
            pos: cell_center(pos) + view.fx.nudge_offset(pos),
            angle: view.facings.angle(pos, cell),
            lift: 0.0,
            stretch: Vec2::ONE,
            moving: false,
            phase: cell_hash(pos) as f32 * 1e-3,
            alpha: 1.0,
        })
        .collect();
    if let Some(handler) = &view.game.animation {
        for m in &handler.moving {
            let t = m.progress;
            let (from, to) = (cell_center(m.from), cell_center(m.to));
            let travels = m.from != m.to;
            let hop = if travels { (PI * t).sin() } else { 0.0 };
            let height = match m.cell {
                Cell::Player(..) => 0.16,
                _ => 0.1,
            };
            creatures.push(Creature {
                cell: m.cell,
                pos: from + (to - from) * ease_in_out(t) + view.fx.nudge_offset(m.to),
                angle: view.facings.angle(m.to, m.cell),
                lift: hop * height,
                stretch: vec2(1.0 - hop * 0.08, 1.0 + hop * 0.14),
                moving: travels,
                phase: cell_hash(m.to) as f32 * 1e-3,
                alpha: 1.0,
            });
        }
    }
    creatures.extend(view.ghosts.iter().map(|&(pos, cell)| Creature {
        cell,
        pos: cell_center(pos),
        angle: facing_angle(cell).unwrap_or(0.0),
        lift: 0.0,
        stretch: Vec2::ONE,
        moving: false,
        phase: 0.0,
        alpha: 0.42,
    }));
    creatures.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));

    let c = layout.space.cell;
    // A warm pool of light around each hero.
    sprites.additive(|| {
        for k in creatures
            .iter()
            .filter(|k| matches!(k.cell, Cell::Player(..)))
        {
            let flicker = 1.0 + (view.time * 7.3 + k.phase).sin() * 0.03;
            sprites.draw_at(
                SpriteId::Glow,
                layout.space.to_screen(k.pos),
                c * 3.4 * flicker,
                Color::new(1.0, 0.78, 0.45, 0.10 * k.alpha),
            );
        }
    });
    for k in &creatures {
        let base = layout.space.to_screen(k.pos);
        let shrink = 1.0 - k.lift * 1.5;
        // The shadow sprite is an ellipse ~0.9 x 0.72 of its canvas.
        sprites.draw(
            SpriteId::Shadow,
            base + vec2(0.0, c * 0.07),
            Vec2::splat(c * 1.05 * shrink),
            0.0,
            Color::new(1.0, 1.0, 1.0, k.alpha),
        );
        if let Cell::Player(player, _) = k.cell {
            let color = player_color(player);
            let pulse = 0.5 + 0.5 * (view.time * 2.5 + k.phase).sin();
            circle(
                base + vec2(0.0, c * 0.04),
                c * (0.36 + pulse * 0.02),
                faded(color, (0.16 + pulse * 0.06) * k.alpha),
            );
        }
    }
    for k in &creatures {
        let base = layout.space.to_screen(k.pos) - vec2(0.0, k.lift * c);
        let breathe = 1.0 + (view.time * 2.6 + k.phase).sin() * 0.015;
        let size = vec2(k.stretch.x, k.stretch.y) * c * CREATURE_SPAN * breathe;
        let sprite = creature_sprite(k.cell, view.time, k.phase, k.moving);
        sprites.draw(
            sprite,
            base,
            size,
            k.angle,
            Color::new(1.0, 1.0, 1.0, k.alpha),
        );
    }
    // Cyber-eye glows, in one pass.
    sprites.additive(|| {
        for k in creatures
            .iter()
            .filter(|k| matches!(k.cell, Cell::CyborgRat(_)))
        {
            let base = layout.space.to_screen(k.pos) - vec2(0.0, k.lift * c);
            let eye = base + vec2(k.angle.sin(), -k.angle.cos()) * c * 0.31;
            sprites.draw_at(
                SpriteId::Glow,
                eye,
                c * 0.42,
                Color::new(1.0, 0.2, 0.2, 0.32 * k.alpha),
            );
        }
    });
}

pub(crate) fn player_color(player: Player) -> Color {
    match player {
        Player::Player1 => palette::PLAYER1,
        Player::Player2 => palette::PLAYER2,
    }
}

fn creature_sprite(cell: Cell, time: f32, phase: f32, moving: bool) -> SpriteId {
    let rate = if moving { 16.0 } else { 6.0 };
    let frame = ((time * rate + phase * 4.0) as usize) % 4;
    match cell {
        Cell::Player(Player::Player1, _) => SpriteId::Hero1,
        Cell::Player(Player::Player2, _) => SpriteId::Hero2,
        Cell::Rat(_) => SpriteId::RAT_FRAMES[frame],
        Cell::CyborgRat(_) => SpriteId::CYBORG_FRAMES[frame],
        _ => unreachable!("not a creature: {cell:?}"),
    }
}

/// Unit vector of a direction, in cells.
pub(crate) fn dir_vector(dir: Dir4) -> Vec2 {
    let PositionDelta { dx, dy } = dir.delta();
    vec2(dx as f32, dy as f32)
}

/// Draw one cell's content statically at `rect` (for the level editor).
pub(crate) fn draw_cell_static(sprites: &Sprites, cell: Cell, rect: Rect, tint: Color) {
    let c = rect.w;
    let center = rect.center();
    let span = Vec2::splat(c * SPRITE_SPAN);
    match cell {
        Cell::Empty => {}
        Cell::Wall => draw_texture_ex(
            &sprites.wall_top,
            rect.x,
            rect.y,
            tint,
            DrawTextureParams {
                dest_size: Some(rect.size()),
                source: Some(Rect::new(0.0, 0.0, TEXEL_CELL, TEXEL_CELL)),
                ..Default::default()
            },
        ),
        Cell::Trigger(digit) => draw_trigger(sprites, center, c, digit, 0.0),
        Cell::Plank => sprites.draw(SpriteId::Plank, center, span, 0.0, tint),
        Cell::Explosive => sprites.draw(SpriteId::Keg, center, span, 0.0, tint),
        Cell::Spiderweb => sprites.draw(SpriteId::Web, center, span, 0.0, tint),
        Cell::BlackHole => {
            sprites.draw(SpriteId::HoleBase, center, span, 0.0, tint);
            sprites.draw(SpriteId::HoleSwirl, center, span, 0.0, tint);
        }
        Cell::Player(..) | Cell::Rat(_) | Cell::CyborgRat(_) => sprites.draw(
            creature_sprite(cell, 0.0, 0.0, false),
            center,
            span,
            facing_angle(cell).unwrap(),
            tint,
        ),
    }
}

pub(crate) fn draw_note_static(sprites: &Sprites, rect: Rect) {
    sprites.draw(
        SpriteId::Note,
        rect.center(),
        Vec2::splat(rect.w * SPRITE_SPAN),
        0.0,
        WHITE,
    );
}
