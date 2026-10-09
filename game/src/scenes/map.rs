//! The world map: the hero walks between levels; cleared levels, open ones
//! and what's still locked are all visible at a glance.

use std::collections::VecDeque;
use std::f32::consts::TAU;

use enum_map::EnumMap;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::atlas::SpriteId;
use crate::audio::Sfx;
use crate::input::{InputHints, MetaInput};
use crate::levels::Level;
use crate::position::Position;
use crate::progress::Progress;
use crate::render::fx::{Blend, BoardSpace, Fx, Particle};
use crate::render::glyphs::{Glyph, PadButton};
use crate::render::palette::{self, GOLD, Palette};
use crate::render::shapes::faded;
use crate::render::terrain::Ground;
use crate::render::ui::{Button, ButtonKind, ScreenLayout};
use crate::scenes::{Ctx, FrameInput, Request};
use crate::world_map::{Theme, WORLD_MAP, WorldMap};

mod draw;

/// Hero walking speed on the map, in cells per second.
const WALK_SPEED: f32 = 9.0;

/// The map's cells: which are floor (and of what region), wall or empty.
struct MapCells {
    size: IVec2,
    ground: Vec<Ground>,
    theme: Vec<Theme>,
}

impl MapCells {
    fn new(map: &WorldMap) -> Self {
        let size = map.size;
        let n = (size.x * size.y) as usize;
        let mut ground = vec![Ground::Void; n];
        let mut theme = vec![Theme::Cellar; n];
        let index = |c: IVec2| (c.y * size.x + c.x) as usize;
        let floors = map
            .regions
            .iter()
            .map(|r| (r.room, r.theme))
            .chain(map.corridors.iter().map(|c| (c.rect, c.theme)));
        for (rect, t) in floors {
            for c in rect.cells() {
                ground[index(c)] = Ground::Floor;
                theme[index(c)] = t;
            }
        }
        for y in 0..size.y {
            for x in 0..size.x {
                let c = ivec2(x, y);
                if ground[index(c)] != Ground::Void {
                    continue;
                }
                let floor_neighbor = (-1..=1)
                    .flat_map(|dy| (-1..=1).map(move |dx| c + ivec2(dx, dy)))
                    .filter(|n| n.x >= 0 && n.y >= 0 && n.x < size.x && n.y < size.y)
                    .find(|&n| ground[index(n)] == Ground::Floor);
                if let Some(n) = floor_neighbor {
                    ground[index(c)] = Ground::Wall;
                    theme[index(c)] = theme[index(n)];
                }
            }
        }
        Self {
            size,
            ground,
            theme,
        }
    }

    fn index(&self, pos: Position) -> Option<usize> {
        (pos.x >= 0 && pos.y >= 0 && pos.x < self.size.x && pos.y < self.size.y)
            .then(|| (pos.y * self.size.x + pos.x) as usize)
    }

    fn ground(&self, pos: Position) -> Ground {
        self.index(pos).map_or(Ground::Void, |i| self.ground[i])
    }

    fn theme(&self, pos: Position) -> Theme {
        self.index(pos).map_or(Theme::Cellar, |i| self.theme[i])
    }
}

/// The edge the hero is walking along.
struct Walk {
    points: Vec<Vec2>,
    travelled: f32,
    to: usize,
}

impl Walk {
    fn length(&self) -> f32 {
        self.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Position and heading at the current distance along the path.
    fn sample(&self) -> (Vec2, Vec2) {
        let mut left = self.travelled;
        for w in self.points.windows(2) {
            let len = w[0].distance(w[1]);
            let dir = (w[1] - w[0]).normalize_or_zero();
            if left <= len {
                return (w[0] + dir * left, dir);
            }
            left -= len;
        }
        let n = self.points.len();
        let dir = (self.points[n - 1] - self.points[n - 2]).normalize_or_zero();
        (self.points[n - 1], dir)
    }
}

pub(crate) struct MapScene {
    /// The node the hero is at (or last left).
    at: usize,
    /// The level the hero is at or heading to.
    selected: usize,
    walk: Option<Walk>,
    /// Nodes to walk through after the current edge.
    route: VecDeque<usize>,
    hero: Vec2,
    hero_angle: f32,
    camera: Vec2,
    time: f32,
    reached: Vec<bool>,
    /// When newly opened nodes pop in (scene time), and whether we've
    /// announced them yet.
    appear: Vec<Option<(f32, bool)>>,
    /// When newly reached regions start shedding their fog.
    reveal: Vec<Option<(f32, bool)>>,
    cells: MapCells,
    palettes: EnumMap<Theme, Palette>,
    fx: Fx,
    /// Enter the selected level as soon as the hero arrives.
    enter_on_arrival: bool,
}

impl MapScene {
    /// The map, with the hero at `from` (or where they last were), and
    /// anything opened since `before` animating in.
    pub(crate) fn new(
        progress: &Progress,
        saved: Option<&str>,
        from: Option<&'static Level>,
        before: Option<&Progress>,
    ) -> Self {
        let map = &*WORLD_MAP;
        let reached = map.reachable(progress);
        let previously = before.map_or_else(|| reached.clone(), |b| map.reachable(b));
        let mut order = 0.0;
        let appear = reached
            .iter()
            .zip(&previously)
            .map(|(&now, &then)| {
                (now && !then).then(|| {
                    order += 1.0;
                    (0.5 + order * 0.3, false)
                })
            })
            .collect();
        let reveal = (0..map.regions.len())
            .map(|r| {
                (map.region_revealed(r, &reached) && !map.region_revealed(r, &previously))
                    .then_some((0.35, false))
            })
            .collect();
        let level_node = |name: &str| map.node_of_level(name).filter(|&n| reached[n]);
        let frontier = || {
            (0..map.nodes.len()).find(|&n| {
                reached[n]
                    && map.nodes[n]
                        .level()
                        .is_some_and(|l| !progress.is_completed(l.name))
            })
        };
        let at = from
            .and_then(|l| level_node(l.name))
            .or_else(|| saved.and_then(level_node))
            .or_else(frontier)
            .unwrap_or(map.start);
        let hero = map.nodes[at].pos;
        Self {
            at,
            selected: at,
            walk: None,
            route: VecDeque::new(),
            hero,
            hero_angle: std::f32::consts::PI,
            camera: hero,
            time: 0.0,
            reached,
            appear,
            reveal,
            cells: MapCells::new(map),
            palettes: EnumMap::from_fn(palette::palette),
            fx: Fx::default(),
            enter_on_arrival: false,
        }
    }

    /// The level selected, if it's a level (not the start).
    pub(crate) fn selected_level(&self) -> Option<&'static Level> {
        WORLD_MAP.nodes[self.selected].level()
    }

    fn space(&self, layout: &ScreenLayout) -> BoardSpace {
        let cell = if layout.portrait() {
            (layout.main.w / 15.0).clamp(22.0, 52.0)
        } else {
            (layout.main.h / 23.0).clamp(24.0, 60.0)
        };
        BoardSpace {
            origin: layout.main.center() - self.camera * cell,
            cell,
        }
    }

    /// Where the hero is on screen, for centering transitions.
    pub(crate) fn focus(&self, layout: &ScreenLayout) -> Vec2 {
        self.space(layout).to_screen(self.hero)
    }

    fn go_to(&mut self, target: usize, ctx: &mut Ctx) {
        let map = &*WORLD_MAP;
        if target == self.selected {
            return;
        }
        self.selected = target;
        let from = self.walk.as_ref().map_or(self.at, |w| w.to);
        self.route = map
            .route(from, target, ctx.progress)
            .into_iter()
            .skip(1)
            .collect();
        if self.walk.is_none() {
            self.next_edge();
        }
        ctx.audio.play(Sfx::UiMove);
    }

    fn next_edge(&mut self) {
        let map = &*WORLD_MAP;
        self.walk = self.route.pop_front().map(|next| Walk {
            points: map.edge_between(self.at, next).points_from(self.at),
            travelled: 0.0,
            to: next,
        });
    }

    pub(crate) fn update(
        &mut self,
        ctx: &mut Ctx,
        input: &FrameInput,
        layout: &ScreenLayout,
    ) -> Option<Request> {
        let map = &*WORLD_MAP;
        let dt = input.dt;
        self.time += dt;
        self.fx.update(dt);
        self.announce(ctx);

        let mut play = false;
        if let Some(dir) = input.nav
            && self.walk.is_none()
        {
            match map.step(self.at, dir, &self.reached) {
                Some(route) => {
                    self.selected = *route.last().expect("a step goes somewhere");
                    self.route = route.into();
                    self.next_edge();
                    ctx.audio.play(Sfx::UiMove);
                }
                None => ctx.audio.play(Sfx::UiLocked),
            }
        }
        if input.confirmed() {
            play = true;
        }
        if input.has(MetaInput::Exit) {
            return Some(Request::Pause);
        }
        let space = self.space(layout);
        for tap in input.taps() {
            if self.play_button(ctx, layout).hit(tap) {
                play = true;
                continue;
            }
            let near = map
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.level().is_some())
                .map(|(i, n)| (i, space.to_screen(n.pos).distance(tap)))
                .filter(|&(_, d)| d < space.cell * 0.8)
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((node, _)) = near {
                if !self.reached[node] {
                    ctx.audio.play(Sfx::UiLocked);
                } else if node == self.selected {
                    play = true;
                } else {
                    self.go_to(node, ctx);
                }
            }
        }
        if play && self.selected_level().is_some() {
            if self.walk.is_none() {
                return self.enter(ctx);
            }
            self.enter_on_arrival = true;
        }

        // Walk along the route.
        let mut remaining = dt * WALK_SPEED * (1.0 + self.route.len() as f32 * 0.35);
        while let Some(walk) = &mut self.walk {
            let length = walk.length();
            walk.travelled += remaining;
            if walk.travelled < length {
                let (pos, dir) = walk.sample();
                self.hero = pos;
                self.hero_angle = dir.x.atan2(-dir.y);
                break;
            }
            remaining = walk.travelled - length;
            self.at = walk.to;
            self.hero = map.nodes[self.at].pos;
            if map.nodes[self.at].level().is_some() {
                ctx.audio.play_at(Sfx::MapStep, 0.9);
            }
            self.next_edge();
        }
        if self.walk.is_none() && self.enter_on_arrival {
            self.enter_on_arrival = false;
            return self.enter(ctx);
        }

        let k = 1.0 - (-dt * 4.0).exp();
        self.camera += (self.hero - self.camera) * k;
        self.sparkle();
        None
    }

    fn enter(&mut self, ctx: &mut Ctx) -> Option<Request> {
        let level = self.selected_level()?;
        ctx.settings.map_level = Some(level.name.to_string());
        ctx.settings.save();
        ctx.audio.play(Sfx::LevelStart);
        Some(Request::PlayLevel(level))
    }

    /// Play sounds as new nodes and regions appear.
    fn announce(&mut self, ctx: &mut Ctx) {
        let map = &*WORLD_MAP;
        for (node, appear) in self.appear.iter_mut().enumerate() {
            if let Some((t, announced)) = appear
                && !*announced
                && self.time >= *t
            {
                *announced = true;
                ctx.audio.play(Sfx::Unlock);
                let at = map.nodes[node].pos;
                for i in 0..12 {
                    let a = i as f32 / 12.0 * TAU;
                    let mut p = Particle::new(SpriteId::Star, at, 0.6);
                    p.vel = Vec2::from_angle(a) * 2.5;
                    p.drag = 3.5;
                    p.size = (0.35, 0.05);
                    p.color = (GOLD, faded(GOLD, 0.0));
                    p.blend = Blend::Additive;
                    self.fx.spawn(p);
                }
            }
        }
        for reveal in &mut self.reveal {
            if let Some((t, announced)) = reveal
                && !*announced
                && self.time >= *t
            {
                *announced = true;
                ctx.audio.play(Sfx::Reveal);
            }
        }
    }

    /// Glints over cleared levels and the hero's light.
    fn sparkle(&mut self) {
        if gen_range(0.0, 1.0) < 0.15 {
            let map = &*WORLD_MAP;
            let node = gen_range(0, map.nodes.len());
            if self.reached[node] && map.nodes[node].level().is_some() {
                let at = map.nodes[node].pos + vec2(gen_range(-0.5, 0.5), gen_range(-0.5, 0.3));
                let mut p = Particle::new(SpriteId::Star, at, 0.8);
                p.vel = vec2(0.0, -0.4);
                p.size = (0.25, 0.0);
                p.color = (Color::new(1.0, 0.95, 0.7, 0.8), faded(WHITE, 0.0));
                p.blend = Blend::Additive;
                self.fx.spawn(p);
            }
        }
    }

    fn play_button(&self, ctx: &Ctx, layout: &ScreenLayout) -> Button<'static> {
        let s = layout.s;
        let rect = if layout.portrait() {
            let p = layout.panel;
            Rect::new(
                p.x + 12.0 * s,
                p.bottom() - 70.0 * s,
                p.w - 24.0 * s,
                58.0 * s,
            )
        } else {
            let p = layout.panel;
            Rect::new(
                p.x + 18.0 * s,
                p.bottom() - 18.0 * s - 60.0 * s,
                p.w - 36.0 * s,
                60.0 * s,
            )
        };
        let glyph = match ctx.hints {
            InputHints::Keyboard => Some(Glyph::Key("Enter")),
            InputHints::Touch => None,
            InputHints::Controller(ty) => Some(Glyph::Pad(ty, PadButton::South)),
        };
        Button {
            rect,
            icon: Some(SpriteId::IconPlay),
            label: "Play",
            glyph,
            kind: ButtonKind::Primary,
            enabled: self.selected_level().is_some(),
            focused: true,
        }
    }
}
