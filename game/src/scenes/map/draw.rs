//! Drawing the world map and its side panel.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use super::MapScene;
use crate::atlas::SpriteId;
use crate::direction::Dir4;
use crate::grid::{Cell, Player};
use crate::input::InputHints;
use crate::levels::Level;
use crate::position::Position;
use crate::progress::Progress;
use crate::render::board::{SPRITE_SPAN, dir_vector};
use crate::render::fx::{BoardSpace, ease_out_back};
use crate::render::palette::{self, DANGER, GOLD, INK, SUCCESS, TEXT, TEXT_DIM, rgb};
use crate::render::shapes::{
    circle, dashed_polyline, ellipse_ring, faded, polyline, radial_gradient, ring, rounded_rect,
    soft_shadow,
};
use crate::render::terrain::{self, Terrain};
use crate::render::text::{self, Align};
use crate::render::ui::{self, ScreenLayout};
use crate::scenes::Ctx;
use crate::sprites::{Face, Sprites};
use crate::world_map::{NodeKind, WORLD_MAP};

impl MapScene {
    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let map = &*WORLD_MAP;
        let space = self.space(layout);
        clear_background(rgb(0x07060a));
        radial_gradient(
            layout.main.center(),
            vec2(screen_width(), screen_height()).length() * 0.7,
            rgb(0x1a1622),
            rgb(0x07060a),
        );

        // Only cells near the screen.
        let view_min = ((vec2(0.0, 0.0) - space.origin) / space.cell).floor() - Vec2::ONE;
        let view_max = ((vec2(screen_width(), screen_height()) - space.origin) / space.cell).ceil()
            + Vec2::ONE;
        let cells: Vec<Position> = (view_min.y.max(0.0) as i32
            ..view_max.y.min(map.size.y as f32) as i32)
            .flat_map(|y| {
                (view_min.x.max(0.0) as i32..view_max.x.min(map.size.x as f32) as i32)
                    .map(move |x| Position { x, y })
            })
            .collect();
        let ground = |p| self.cells.ground(p);
        let palette = |p| &self.palettes[self.cells.theme(p)];
        let rise = |_| None;
        let terrain = Terrain {
            space,
            ground: &ground,
            palette: &palette,
            rise: &rise,
        };
        // The keep floats on a soft shadow.
        for region in &map.regions {
            let r = region.room.rect();
            let rect = Rect::new(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0);
            let tl = space.to_screen(rect.point());
            soft_shadow(
                Rect::new(
                    tl.x + space.cell * 0.2,
                    tl.y + space.cell * 0.6,
                    rect.w * space.cell,
                    rect.h * space.cell,
                ),
                space.cell * 0.4,
                space.cell * 1.2,
                Color::new(0.0, 0.0, 0.0, 0.5),
            );
        }
        terrain::draw_floor(sprites, &terrain, &cells);
        terrain::draw_ambient_occlusion(&terrain, &cells);
        terrain::draw_walls(sprites, &terrain, &cells);
        terrain::draw_outlines(&terrain, &cells);

        self.draw_paths(space);
        for prop in &map.props {
            let at = space.to_screen(prop.pos());
            sprites.draw(
                SpriteId::Shadow,
                at + vec2(0.0, space.cell * 0.28),
                vec2(space.cell * 0.9, space.cell * 0.45) * prop.scale,
                0.0,
                faded(WHITE, 0.7),
            );
            sprites.draw_at(
                prop.sprite,
                at,
                space.cell * SPRITE_SPAN * prop.scale,
                WHITE,
            );
        }
        self.draw_chains(space, ctx.progress);
        self.draw_nodes(sprites, space, ctx.progress);
        self.draw_hero(sprites, space);
        if ctx.hints != InputHints::Touch {
            self.draw_exits(space);
        }
        let labels = self.labels(sprites, space, layout.s);
        self.fx.draw_over(sprites, space);
        self.draw_banners(sprites, space, layout.s, &labels);
        for label in &labels {
            draw_label(sprites, label, layout.s);
        }
        self.draw_fog(sprites, space, layout.s);
        self.draw_panel(ctx, layout);
    }
    pub(super) fn node_alpha(&self, node: usize) -> f32 {
        match self.appear[node] {
            Some((t, _)) => ease_out_back(((self.time - t) / 0.45).clamp(0.0, 1.0)),
            None => 1.0,
        }
    }
    fn draw_paths(&self, space: BoardSpace) {
        let map = &*WORLD_MAP;
        let c = space.cell;
        for edge in &map.edges {
            let [a, b] = edge.ends;
            let open = self.reached[a] && self.reached[b];
            let shown = self.reached[a] || self.reached[b];
            if !shown {
                continue;
            }
            let alpha = self.node_alpha(a).min(self.node_alpha(b)).clamp(0.0, 1.0);
            let points: Vec<Vec2> = edge.points.iter().map(|&p| space.to_screen(p)).collect();
            if open {
                dashed_polyline(
                    &points,
                    c * 0.2,
                    c * 0.24,
                    c * 0.16,
                    0.0,
                    faded(INK, 0.6 * alpha),
                );
                dashed_polyline(
                    &points,
                    c * 0.12,
                    c * 0.24,
                    c * 0.16,
                    0.0,
                    faded(rgb(0xf3e3b8), 0.95 * alpha),
                );
            } else {
                dashed_polyline(
                    &points,
                    c * 0.1,
                    c * 0.1,
                    c * 0.2,
                    0.0,
                    faded(rgb(0xf3e3b8), 0.25),
                );
            }
        }
        // Plazas where paths meet.
        for node in &map.nodes {
            if let NodeKind::Junction = node.kind {
                let at = space.to_screen(node.pos);
                circle(at, c * 0.42, faded(INK, 0.5));
                circle(at, c * 0.34, rgb(0xd9c79a));
                ring(at, c * 0.22, c * 0.05, faded(INK, 0.4));
            }
        }
    }
    fn draw_nodes(&self, sprites: &Sprites, space: BoardSpace, progress: &Progress) {
        let map = &*WORLD_MAP;
        let c = space.cell;
        for (i, node) in map.nodes.iter().enumerate() {
            let Some(level) = node.level() else {
                continue;
            };
            if !map.region_revealed(node.region, &self.reached) {
                continue;
            }
            let at = space.to_screen(node.pos);
            let pop = self.node_alpha(i);
            let size = c * 1.5 * pop.max(0.0);
            let completed = self.reached[i] && progress.is_completed(level.name);
            let (base, icon, tint) = if !self.reached[i] {
                (SpriteId::NodeLocked, SpriteId::IconLock, rgb(0x9a93a8))
            } else if completed {
                (SpriteId::NodeDone, SpriteId::IconCheck, rgb(0x2f6b3a))
            } else {
                (SpriteId::NodeOpen, SpriteId::IconRat, rgb(0x5a1a24))
            };
            if i == self.selected {
                let pulse = 0.5 + 0.5 * (self.time * 4.0).sin();
                sprites.additive(|| {
                    sprites.draw_at(
                        SpriteId::Glow,
                        at,
                        c * (2.6 + pulse * 0.3),
                        faded(GOLD, 0.5),
                    )
                });
            } else if self.reached[i] && !completed {
                let pulse = 0.5 + 0.5 * (self.time * 2.5 + i as f32).sin();
                sprites.additive(|| {
                    sprites.draw_at(
                        SpriteId::Glow,
                        at,
                        c * 2.0 * pop.max(0.0),
                        faded(DANGER, 0.15 + pulse * 0.2),
                    )
                });
            }
            sprites.draw(
                SpriteId::Shadow,
                at + vec2(0.0, c * 0.3),
                vec2(c * 1.2, c * 0.55) * pop.max(0.0),
                0.0,
                faded(WHITE, 0.7),
            );
            sprites.draw_at(base, at, size, WHITE);
            // The medallion's face sits a little above its canvas center.
            let face = at - vec2(0.0, size * 6.0 / 256.0);
            let bounce = if self.reached[i] && !completed {
                1.0 + (self.time * 3.0 + i as f32).sin() * 0.06
            } else {
                1.0
            };
            sprites.draw_at(icon, face, size * 0.42 * bounce, tint);
            if level.grid.find_players().len() > 1 {
                sprites.draw_at(
                    SpriteId::IconPlayers,
                    at + vec2(c * 0.5, c * 0.45),
                    c * 0.42,
                    palette::PLAYER2,
                );
            }
        }
    }
    /// Name tags for the selected level and the one under the pointer.
    fn labels(&self, sprites: &Sprites, space: BoardSpace, s: f32) -> Vec<Label> {
        let map = &*WORLD_MAP;
        let hovered = ui::hover_pos().and_then(|p| {
            map.nodes.iter().position(|n| {
                n.level().is_some() && space.to_screen(n.pos).distance(p) < space.cell * 0.8
            })
        });
        let mut labels = Vec::new();
        for node in [Some(self.selected), hovered].into_iter().flatten() {
            let Some(level) = map.nodes[node].level() else {
                continue;
            };
            if !map.region_revealed(map.nodes[node].region, &self.reached) {
                continue;
            }
            let requires = &map.nodes[node].requires;
            let name = if self.reached[node] {
                level.display_name.clone()
            } else if requires.is_empty() {
                "Locked".to_string()
            } else {
                let names: Vec<&str> = requires.iter().map(|l| l.display_name.as_str()).collect();
                match names.split_last() {
                    Some((last, [])) => format!("Needs {last}"),
                    Some((last, rest)) => format!("Needs {} & {last}", rest.join(", ")),
                    None => unreachable!("requirements are non-empty"),
                }
            };
            let w = text::width(sprites, &name, Face::Display, LABEL_SIZE * s) + 20.0 * s;
            let h = 28.0 * s;
            // Above the medallion, unless that's where an exit arrow is.
            let center = space.to_screen(map.nodes[node].pos);
            let offset = space.cell * 1.15 + h / 2.0;
            let anchor = match self.exit_arrows_at(node) {
                (true, false) => center + vec2(0.0, offset),
                (true, true) => center - vec2(0.0, offset + space.cell * 0.9),
                (false, _) => center - vec2(0.0, offset),
            };
            labels.push(Label {
                rect: Rect::new(anchor.x - w / 2.0, anchor.y - h / 2.0, w, h),
                name,
                selected: node == self.selected,
            });
        }
        labels
    }
    /// Chains from each level still to be cleared to the node it keeps
    /// locked, plus any chains shattering because they just came free.
    fn draw_chains(&self, space: BoardSpace, progress: &Progress) {
        let map = &*WORLD_MAP;
        let shown =
            |[_, locked]: [usize; 2]| map.region_revealed(map.nodes[locked].region, &self.reached);
        let ends = |[key, locked]: [usize; 2]| (map.nodes[key].pos, map.nodes[locked].pos);
        for chain in map.chains(progress).into_iter().filter(|&c| shown(c)) {
            let (from, to) = ends(chain);
            draw_chain(space, from, to, 0.0);
        }
        for &(chain, start, _) in &self.breaking {
            let burst = ((self.time - start) / 0.7).clamp(0.0, 1.0);
            if burst < 1.0 && shown(chain) {
                let (from, to) = ends(chain);
                draw_chain(space, from, to, burst);
            }
        }
    }

    /// Whether exit arrows point (up, down) from `node` right now.
    fn exit_arrows_at(&self, node: usize) -> (bool, bool) {
        if node != self.at || self.walk.is_some() {
            return (false, false);
        }
        let exits = WORLD_MAP.exits(node);
        let open = |dir: Dir4| exits[dir].is_some_and(|next| self.reached[next]);
        (open(Dir4::North), open(Dir4::South))
    }

    /// While the hero stands still, an arrow on each open path out saying
    /// which way to press to take it.
    fn draw_exits(&self, space: BoardSpace) {
        if self.walk.is_some() {
            return;
        }
        let map = &*WORLD_MAP;
        let c = space.cell;
        let pulse = 1.0 + (self.time * 4.0).sin() * 0.06;
        for (dir, next) in map.exits(self.at) {
            let Some(next) = next else {
                continue;
            };
            // Not toward locked nodes, nor ones yet to pop in.
            if !self.reached[next] || self.node_alpha(next) <= 0.0 {
                continue;
            }
            // Partway along the path toward its next stop.
            let points = map.edge_between(self.at, next).points_from(self.at);
            let toward = (points[1] - points[0]).normalize();
            let at = space.to_screen(points[0] + toward * 1.25);
            let r = c * 0.36 * pulse;
            circle(at + vec2(0.0, c * 0.05), r, faded(INK, 0.6));
            circle(at, r, GOLD);
            ring(at, r, c * 0.05, INK);
            let d = dir_vector(dir);
            let side = vec2(-d.y, d.x);
            let tip = at + d * r * 0.55;
            let back = at - d * r * 0.35;
            draw_triangle(tip, back + side * r * 0.5, back - side * r * 0.5, INK);
        }
    }

    fn draw_hero(&self, sprites: &Sprites, space: BoardSpace) {
        let c = space.cell;
        let walking = self.walk.is_some();
        let bob = if walking {
            (self.time * 18.0).sin().abs() * 0.12
        } else {
            0.0
        };
        let at = space.to_screen(self.hero);
        sprites.draw(
            SpriteId::Shadow,
            at + vec2(0.0, c * 0.12),
            vec2(c * 0.8, c * 0.5),
            0.0,
            WHITE,
        );
        circle(at, c * 0.42, faded(palette::PLAYER1, 0.25));
        sprites.draw(
            SpriteId::Hero1,
            at - vec2(0.0, bob * c + c * 0.15),
            Vec2::splat(c * SPRITE_SPAN * 1.05),
            self.hero_angle,
            WHITE,
        );
    }
    fn draw_banners(&self, sprites: &Sprites, space: BoardSpace, s: f32, labels: &[Label]) {
        let map = &*WORLD_MAP;
        for (i, region) in map.regions.iter().enumerate() {
            if !map.region_revealed(i, &self.reached) {
                continue;
            }
            let r = region.room.rect();
            let at = space.to_screen(vec2(r.center().x, r.y - 0.5));
            let size = (space.cell * 0.55).max(15.0 * s);
            let w = text::width(sprites, &region.name, Face::Display, size) + size * 1.6;
            let h = size * 1.6;
            let banner = Rect::new(at.x - w / 2.0, at.y - h / 2.0, w, h);
            // Make way for a level's name tag.
            let alpha = if labels.iter().any(|l| l.rect.overlaps(&banner)) {
                0.15
            } else {
                1.0
            };
            rounded_rect(
                Rect::new(banner.x, banner.y + 3.0, banner.w, banner.h),
                h * 0.3,
                faded(INK, 0.6 * alpha),
            );
            let color = self.palettes[region.theme].motes;
            rounded_rect(banner, h * 0.3, faded(rgb(0x2a2234), alpha));
            crate::render::shapes::rounded_rect_outline(
                banner,
                h * 0.3,
                2.0,
                faded(color, 0.8 * alpha),
            );
            text::draw_aligned(
                sprites,
                &region.name,
                banner.center(),
                Align::Center,
                Face::Display,
                size,
                faded(TEXT, alpha),
            );
        }
    }
    /// Unexplored regions hide under dark clouds, which clear when reached.
    fn draw_fog(&self, sprites: &Sprites, space: BoardSpace, s: f32) {
        let map = &*WORLD_MAP;
        for (i, region) in map.regions.iter().enumerate() {
            let revealed = map.region_revealed(i, &self.reached);
            let fade = match self.reveal[i] {
                Some((t, _)) => 1.0 - ((self.time - t) / 1.4).clamp(0.0, 1.0),
                None if revealed => 0.0,
                None => 1.0,
            };
            if fade <= 0.0 {
                continue;
            }
            let r = region.room.rect();
            let tl = space.to_screen(vec2(r.x - 1.2, r.y - 1.2));
            let rect = Rect::new(
                tl.x,
                tl.y,
                (r.w + 2.4) * space.cell,
                (r.h + 2.4) * space.cell,
            );
            soft_shadow(
                rect,
                space.cell,
                space.cell * 1.5,
                Color::new(0.03, 0.025, 0.05, 0.96 * fade),
            );
            for k in 0..10 {
                let f = k as f32;
                let drift = (self.time * 0.15 + f * 1.7).sin() * 0.8;
                let p = vec2(
                    r.x + (f * 0.37).fract() * r.w + drift,
                    r.y + (f * 0.61).fract() * r.h,
                );
                sprites.draw_at(
                    SpriteId::Puff,
                    space.to_screen(p),
                    space.cell * (4.0 + (f * 0.7).sin() * 1.5) * (0.6 + 0.4 * fade),
                    Color::new(0.12, 0.1, 0.16, 0.5 * fade),
                );
            }
            let center = space.to_screen(r.center());
            sprites.draw_at(
                SpriteId::IconLock,
                center - vec2(0.0, 22.0 * s),
                36.0 * s,
                faded(TEXT_DIM, 0.7 * fade),
            );
            text::draw_aligned(
                sprites,
                if revealed { &region.name } else { "Unexplored" },
                center + vec2(0.0, 16.0 * s),
                Align::Center,
                Face::Display,
                20.0 * s,
                faded(TEXT_DIM, 0.8 * fade),
            );
        }
    }
    fn draw_panel(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let s = layout.s;
        let map = &*WORLD_MAP;
        let node = &map.nodes[self.selected];
        let region = &map.regions[node.region];
        let level = node.level();
        let p = layout.panel;
        ui::panel(p, s);
        let levels = map.nodes.iter().filter_map(|n| n.level());
        let total = levels.clone().count();
        let done = levels.filter(|l| ctx.progress.is_completed(l.name)).count();
        let cleared = format!("{done} / {total} cleared");
        if let Some(top) = layout.top {
            ui::panel(top, s);
            text::draw_aligned(
                sprites,
                "THE KEEP",
                vec2(top.x + 16.0 * s, top.center().y),
                Align::Left,
                Face::Display,
                22.0 * s,
                GOLD,
            );
            text::draw_aligned(
                sprites,
                &cleared,
                vec2(top.right() - 16.0 * s, top.center().y),
                Align::Right,
                Face::Display,
                17.0 * s,
                TEXT_DIM,
            );
        } else {
            text::draw_aligned(
                sprites,
                &cleared,
                vec2(p.right() - 18.0 * s, p.y + 26.0 * s),
                Align::Right,
                Face::Display,
                15.0 * s,
                GOLD,
            );
        }
        let pad = 18.0 * s;
        let x = p.x + pad;
        let w = p.w - 2.0 * pad;
        let mut y = p.y + pad + 8.0 * s;
        ui::heading(sprites, &region.name.to_uppercase(), vec2(x, y), s);
        y += 30.0 * s;
        let name = level.map_or("Crossroads", |l| l.display_name.as_str());
        let size = (34.0 * s)
            .min(34.0 * s * w / text::width(sprites, name, Face::Display, 34.0 * s).max(1.0));
        text::draw_aligned(
            sprites,
            name,
            vec2(x, y),
            Align::Left,
            Face::Display,
            size,
            TEXT,
        );
        y += 32.0 * s;
        if let Some(level) = level {
            let (icon, label, color) = if ctx.progress.is_completed(level.name) {
                (SpriteId::IconCheck, "Cleared", SUCCESS)
            } else {
                (SpriteId::IconRat, "Infested", DANGER)
            };
            sprites.draw_at(icon, vec2(x + 11.0 * s, y), 22.0 * s, color);
            text::draw_aligned(
                sprites,
                label,
                vec2(x + 28.0 * s, y),
                Align::Left,
                Face::Display,
                18.0 * s,
                color,
            );
            if level.grid.find_players().len() > 1 {
                sprites.draw_at(
                    SpriteId::IconPlayers,
                    vec2(x + 130.0 * s, y),
                    24.0 * s,
                    palette::PLAYER2,
                );
                text::draw_aligned(
                    sprites,
                    "2 players",
                    vec2(x + 148.0 * s, y),
                    Align::Left,
                    Face::Display,
                    18.0 * s,
                    palette::PLAYER2,
                );
            }
            y += 22.0 * s;
            let button_top = self.play_button(ctx, layout).rect.y;
            let note_h = if region.note.is_some() && !layout.portrait() {
                96.0 * s
            } else {
                0.0
            };
            let preview = Rect::new(x, y, w, (button_top - y - 16.0 * s - note_h).max(0.0));
            if preview.h > 40.0 * s {
                draw_preview(level, preview, s);
            }
            if let Some(note) = &region.note
                && note_h > 0.0
            {
                let mut ny = button_top - note_h + 4.0 * s;
                for line in text::wrap(sprites, note, Face::Body, 17.0 * s, w) {
                    text::draw_aligned(
                        sprites,
                        &line,
                        vec2(x, ny),
                        Align::Left,
                        Face::Body,
                        17.0 * s,
                        TEXT_DIM,
                    );
                    ny += 21.0 * s;
                }
            }
        }
        self.play_button(ctx, layout).draw(sprites, s);
    }
}

/// Text size of a level's name tag, before scaling.
const LABEL_SIZE: f32 = 17.0;

/// A level's name tag on the map.
struct Label {
    rect: Rect,
    name: String,
    /// Whether it's for the selected level (rather than one under the pointer).
    selected: bool,
}

fn draw_label(sprites: &Sprites, label: &Label, s: f32) {
    let rect = label.rect;
    rounded_rect(
        Rect::new(rect.x, rect.y + 3.0 * s, rect.w, rect.h),
        rect.h / 2.0,
        faded(INK, 0.5),
    );
    rounded_rect(rect, rect.h / 2.0, rgb(0x231e2e));
    crate::render::shapes::rounded_rect_outline(
        rect,
        rect.h / 2.0,
        1.5 * s,
        if label.selected {
            GOLD
        } else {
            faded(TEXT_DIM, 0.6)
        },
    );
    text::draw_aligned(
        sprites,
        &label.name,
        rect.center(),
        Align::Center,
        Face::Display,
        LABEL_SIZE * s,
        TEXT,
    );
}

/// A miniature of a level's layout.
fn draw_preview(level: &Level, area: Rect, s: f32) {
    let grid = &level.grid;
    let (w, h) = (grid.width() as f32, grid.height() as f32);
    let cell = (area.w / w).min(area.h / h).min(18.0 * s);
    let size = vec2(w, h) * cell;
    let origin = vec2(
        area.center().x - size.x / 2.0,
        area.y + (area.h - size.y) / 2.0,
    );
    let frame = Rect::new(
        origin.x - 6.0 * s,
        origin.y - 6.0 * s,
        size.x + 12.0 * s,
        size.y + 12.0 * s,
    );
    ui::card(frame, s);
    for (pos, c) in grid.entries() {
        let p = origin + vec2(pos.x as f32, pos.y as f32) * cell;
        let r = Rect::new(p.x, p.y, cell, cell);
        let center = r.center();
        let floor = rgb(0x4a4456);
        match c {
            Cell::Wall => draw_rectangle(r.x, r.y, r.w, r.h, rgb(0x1a1621)),
            _ => draw_rectangle(r.x, r.y, r.w, r.h, floor),
        }
        let dot = |color: Color, k: f32| circle(center, cell * k, color);
        match c {
            Cell::Player(Player::Player1, _) => dot(palette::PLAYER1, 0.45),
            Cell::Player(Player::Player2, _) => dot(palette::PLAYER2, 0.45),
            Cell::Rat(_) => dot(rgb(0xc98f6a), 0.36),
            Cell::CyborgRat(_) => dot(rgb(0x6ef0ff), 0.38),
            Cell::Plank => draw_rectangle(
                r.x + cell * 0.15,
                r.y + cell * 0.3,
                cell * 0.7,
                cell * 0.4,
                rgb(0xa0683a),
            ),
            Cell::Spiderweb => dot(faded(WHITE, 0.45), 0.4),
            Cell::BlackHole => dot(rgb(0x8a3cc9), 0.42),
            Cell::Explosive => dot(rgb(0xe0453a), 0.38),
            Cell::Trigger(d) => dot(palette::trigger(d), 0.3),
            Cell::Wall | Cell::Empty => {}
        }
    }
}

/// An iron chain hanging slightly slack from `from` to `to` (map cells).
/// `burst` in [0, 1] flings the links apart as it shatters.
fn draw_chain(space: BoardSpace, from: Vec2, to: Vec2, burst: f32) {
    let c = space.cell;
    let length = from.distance(to);
    let middle = (from + to) / 2.0 + vec2(0.0, (length * 0.06).min(0.8));
    let point = |t: f32| {
        let u = 1.0 - t;
        from * u * u + middle * 2.0 * u * t + to * t * t
    };
    let link = 0.3;
    let count = (length / (link * 0.8)).ceil().max(2.0) as usize;
    let alpha = 1.0 - burst;
    // Each link's screen position and angle.
    let links: Vec<(Vec2, f32)> = (0..count)
        .map(|i| {
            let t = (i as f32 + 0.5) / count as f32;
            let tangent = point((t + 0.01).min(1.0)) - point((t - 0.01).max(0.0));
            // A stable random direction per link to fly off in.
            let random = ((i as f32 * 12.9898).sin() * 43_758.547).fract().abs();
            let fling = Vec2::from_angle(random * TAU) * burst * 1.4 + vec2(0.0, burst * burst);
            let angle = tangent.y.atan2(tangent.x) + (random - 0.5) * 8.0 * burst;
            (space.to_screen(point(t) + fling), angle)
        })
        .collect();
    // Links alternate between face-on rings and edge-on bars.
    let draw_links = |offset: Vec2, thickness: f32, color: &dyn Fn(bool) -> Color| {
        for (i, &(center, angle)) in links.iter().enumerate() {
            let center = center + offset;
            if i % 2 == 0 {
                ellipse_ring(
                    center,
                    vec2(link * 0.55, link * 0.3) * c,
                    angle,
                    thickness,
                    color(true),
                );
            } else {
                let d = Vec2::from_angle(angle) * link * 0.45 * c;
                polyline(&[center - d, center + d], thickness * 1.2, color(false));
            }
        }
    };
    draw_links(vec2(0.05, 0.09) * c, c * 0.1, &|_| {
        Color::new(0.0, 0.0, 0.0, 0.3 * alpha)
    });
    draw_links(Vec2::ZERO, c * 0.1, &|_| faded(INK, alpha));
    draw_links(Vec2::ZERO, c * 0.055, &|face_on| {
        faded(
            if face_on {
                rgb(0x8d8999)
            } else {
                rgb(0xbdb8c9)
            },
            alpha,
        )
    });
}
