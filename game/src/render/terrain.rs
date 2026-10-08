//! Stone floors and walls on a cell grid, shared by levels and the world
//! map: flagstone floors, raised walls with a visible south face, soft
//! shadows where they meet, and ink outlines.

use macroquad::prelude::*;

use crate::direction::Dir4;
use crate::position::{Position, PositionDelta};
use crate::render::fx::BoardSpace;
use crate::render::palette::{INK, Palette};
use crate::render::shapes::{gradient_quad, vertical_gradient};
use crate::sprites::Sprites;

/// Height of a wall's south face, as a fraction of the cell.
pub(crate) const WALL_FRONT: f32 = 0.30;
/// Texture pixels per cell for the tiling textures.
pub(crate) const TEXEL_CELL: f32 = 256.0;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Ground {
    Floor,
    Wall,
    /// Outside the playable area (the world map's dark surroundings).
    Void,
}

/// A grid of ground to draw.
pub(crate) struct Terrain<'a> {
    pub(crate) space: BoardSpace,
    pub(crate) ground: &'a dyn Fn(Position) -> Ground,
    /// Colors for the cell's region.
    pub(crate) palette: &'a dyn Fn(Position) -> &'a Palette,
    /// How far a newly raised wall has grown, if it's still rising.
    pub(crate) rise: &'a dyn Fn(Position) -> Option<f32>,
}

impl Terrain<'_> {
    fn rect(&self, pos: Position) -> Rect {
        let p = self.space.to_screen(vec2(pos.x as f32, pos.y as f32));
        Rect::new(p.x, p.y, self.space.cell, self.space.cell)
    }

    fn is(&self, pos: Position, ground: Ground) -> bool {
        (self.ground)(pos) == ground
    }
}

/// A stable pseudo-random number per cell, for visual variety.
pub(crate) fn cell_hash(pos: Position) -> u32 {
    let mut h = (pos.x as u32).wrapping_mul(0x9e37_79b1) ^ (pos.y as u32).wrapping_mul(0x85eb_ca77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

pub(crate) fn darken(color: Color, k: f32) -> Color {
    Color::new(color.r * k, color.g * k, color.b * k, color.a)
}

/// Flagstones under every non-void cell (walls included, in case they're
/// rising out of it).
pub(crate) fn draw_floor(sprites: &Sprites, t: &Terrain, cells: &[Position]) {
    for &pos in cells {
        if t.is(pos, Ground::Void) {
            continue;
        }
        let r = t.rect(pos);
        let h = cell_hash(pos);
        let tile = h % 16;
        draw_texture_ex(
            &sprites.floor,
            r.x,
            r.y,
            (t.palette)(pos).floor,
            DrawTextureParams {
                dest_size: Some(r.size()),
                source: Some(Rect::new(
                    (tile % 4) as f32 * TEXEL_CELL,
                    (tile / 4) as f32 * TEXEL_CELL,
                    TEXEL_CELL,
                    TEXEL_CELL,
                )),
                flip_x: h & 0x100 != 0,
                flip_y: h & 0x200 != 0,
                ..Default::default()
            },
        );
    }
}

/// Soft shadows on the floor where it meets walls.
pub(crate) fn draw_ambient_occlusion(t: &Terrain, cells: &[Position]) {
    let c = t.space.cell;
    let dark = |a: f32| Color::new(0.03, 0.02, 0.06, a);
    let clear = dark(0.0);
    for &pos in cells {
        if !t.is(pos, Ground::Floor) {
            continue;
        }
        let r = t.rect(pos);
        let (tl, tr, br, bl) = (
            vec2(r.x, r.y),
            vec2(r.right(), r.y),
            vec2(r.right(), r.bottom()),
            vec2(r.x, r.bottom()),
        );
        let wall = |dx, dy| t.is(pos + PositionDelta::new(dx, dy), Ground::Wall);
        // The wall to the north stands tallest over this cell.
        if wall(0, -1) {
            let d = vec2(0.0, c * 0.34);
            gradient_quad([tl, tr, tr + d, tl + d], dark(0.5), clear);
        }
        if wall(0, 1) {
            let d = vec2(0.0, -c * 0.16);
            gradient_quad([bl, br, br + d, bl + d], dark(0.28), clear);
        }
        if wall(-1, 0) {
            let d = vec2(c * 0.22, 0.0);
            gradient_quad([tl, bl, bl + d, tl + d], dark(0.36), clear);
        }
        if wall(1, 0) {
            let d = vec2(-c * 0.22, 0.0);
            gradient_quad([tr, br, br + d, tr + d], dark(0.36), clear);
        }
    }
}

/// Raised walls: a textured top, and a south face where the wall's south
/// side is open.
pub(crate) fn draw_walls(sprites: &Sprites, t: &Terrain, cells: &[Position]) {
    let c = t.space.cell;
    for &pos in cells {
        if !t.is(pos, Ground::Wall) {
            continue;
        }
        let palette = (t.palette)(pos);
        let full = t.rect(pos);
        let r = match (t.rise)(pos) {
            Some(p) => Rect::new(full.x, full.bottom() - full.h * p, full.w, full.h * p),
            None => full,
        };
        let south = pos + Dir4::South.delta();
        let south_open = !t.is(south, Ground::Wall);
        let front_h = if south_open { WALL_FRONT * r.h } else { 0.0 };
        let top = Rect::new(r.x, r.y, r.w, r.h - front_h);
        draw_texture_ex(
            &sprites.wall_top,
            top.x,
            top.y,
            palette.wall,
            DrawTextureParams {
                dest_size: Some(top.size()),
                source: Some(Rect::new(
                    pos.x as f32 * TEXEL_CELL,
                    pos.y as f32 * TEXEL_CELL,
                    TEXEL_CELL,
                    TEXEL_CELL * top.h / full.h,
                )),
                ..Default::default()
            },
        );
        if !t.is(pos + Dir4::North.delta(), Ground::Wall) {
            // Light catching the top edge.
            draw_rectangle(
                top.x,
                top.y,
                top.w,
                c * 0.06,
                Color::new(1.0, 1.0, 1.0, 0.16),
            );
        }
        if south_open {
            let shade = if t.is(south, Ground::Void) { 0.6 } else { 0.9 };
            draw_wall_front(
                sprites,
                Rect::new(r.x, top.bottom(), r.w, front_h),
                pos.x as f32,
                darken(palette.wall, shade),
            );
        }
    }
}

/// The south face of a wall: a strip of stone courses darkening toward the
/// base. `x` aligns the texture with the cell column.
pub(crate) fn draw_wall_front(sprites: &Sprites, rect: Rect, x: f32, tint: Color) {
    draw_texture_ex(
        &sprites.wall_front,
        rect.x,
        rect.y,
        tint,
        DrawTextureParams {
            dest_size: Some(rect.size()),
            source: Some(Rect::new(
                x * TEXEL_CELL,
                0.0,
                TEXEL_CELL,
                128.0 * WALL_FRONT / 0.5,
            )),
            ..Default::default()
        },
    );
    vertical_gradient(
        rect,
        Color::new(0.0, 0.0, 0.0, 0.0),
        Color::new(0.0, 0.0, 0.0, 0.35),
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h * 0.1,
        Color::new(1.0, 1.0, 1.0, 0.12),
    );
}

/// Ink lines along every edge where a wall meets anything else.
pub(crate) fn draw_outlines(t: &Terrain, cells: &[Position]) {
    let c = t.space.cell;
    let w = (c * 0.055).max(1.5);
    for &pos in cells {
        if !t.is(pos, Ground::Wall) {
            continue;
        }
        let r = t.rect(pos);
        let open = |d: Dir4| !t.is(pos + d.delta(), Ground::Wall);
        let ext = w / 2.0;
        if open(Dir4::North) {
            draw_rectangle(r.x - ext, r.y - w / 2.0, r.w + w, w, INK);
        }
        if open(Dir4::South) {
            draw_rectangle(r.x - ext, r.bottom() - w / 2.0, r.w + w, w, INK);
        }
        if open(Dir4::West) {
            draw_rectangle(r.x - w / 2.0, r.y - ext, w, r.h + w, INK);
        }
        if open(Dir4::East) {
            draw_rectangle(r.right() - w / 2.0, r.y - ext, w, r.h + w, INK);
        }
    }
}
