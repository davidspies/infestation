//! The heads-up display: sidebar or bottom bar, controls reference, field
//! guide, dialogue, level intro banner and the win/lose cards.

use macroquad::prelude::*;

use super::{LevelScene, OUTCOME_DELAY, UiAction};
use crate::atlas::SpriteId;
use crate::game::{Death, PlayState};
use crate::grid::{Cell, Player};
use crate::input::InputHints;
use crate::levels::Level;
use crate::render::board::{self, BoardLayout};
use crate::render::glyphs::{Glyph, PadButton};
use crate::render::palette::{self, DANGER, GOLD, INK, TEXT, TEXT_DIM};
use crate::render::shapes::{faded, rounded_rect};
use crate::render::text::{self, Align};
use crate::render::ui::{self, Button, ButtonKind, ScreenLayout};
use crate::scenes::Ctx;
use crate::sprites::{Face, Sprites};

impl LevelScene {
    /// Asks for a second press before restarting throws away the moves.
    pub(super) fn draw_restart_prompt(&self, ctx: &Ctx, board: BoardLayout, s: f32) {
        let sprites = ctx.sprites;
        let slab = board.slab_rect();
        let h = 44.0 * s;
        let (before, glyph, after) = match ctx.hints {
            InputHints::Keyboard => ("Press", Some(Glyph::Key("R")), "again to restart"),
            InputHints::Controller(ty) => (
                "Press",
                Some(Glyph::Pad(ty, PadButton::LeftShoulder)),
                "again to restart",
            ),
            InputHints::Touch => ("Tap Restart again to start over", None, ""),
        };
        let size = 19.0 * s;
        let gap = 8.0 * s;
        let glyph_h = 28.0 * s;
        let glyph_w = glyph.map_or(0.0, |g| {
            crate::render::glyphs::width(sprites, g, glyph_h) + gap
        });
        let w = text::width(sprites, before, Face::Display, size)
            + gap
            + glyph_w
            + text::width(sprites, after, Face::Display, size)
            + 36.0 * s;
        // Straddling the slab's bottom edge, clear of the playfield.
        let y = (slab.bottom() - h / 2.0).min(screen_height() - h - 6.0 * s);
        let rect = Rect::new(slab.center().x - w / 2.0, y, w, h);
        rounded_rect(
            Rect::new(rect.x, rect.y + 3.0 * s, rect.w, rect.h),
            h / 2.0,
            faded(INK, 0.6),
        );
        rounded_rect(rect, h / 2.0, palette::PANEL);
        crate::render::shapes::rounded_rect_outline(rect, h / 2.0, 2.0 * s, GOLD);
        let mut x = rect.x + 18.0 * s;
        let cy = rect.center().y;
        text::draw_aligned(
            sprites,
            before,
            vec2(x, cy),
            Align::Left,
            Face::Display,
            size,
            TEXT,
        );
        x += text::width(sprites, before, Face::Display, size) + gap;
        if let Some(g) = glyph {
            x += crate::render::glyphs::draw(sprites, g, vec2(x, cy), glyph_h) + gap;
        }
        text::draw_aligned(
            sprites,
            after,
            vec2(x, cy),
            Align::Left,
            Face::Display,
            size,
            TEXT,
        );
    }

    pub(super) fn draw_intro(&self, sprites: &Sprites, board: BoardLayout, s: f32) {
        let t = self.time;
        if t > 2.2 {
            return;
        }
        let alpha = (t / 0.25).min(1.0) * ((2.2 - t) / 0.5).min(1.0);
        let slab = board.slab_rect();
        let y = slab.y + slab.h * 0.32 - (1.0 - (t / 0.4).min(1.0)).powi(3) * 30.0 * s;
        let band = Rect::new(slab.x, y - 46.0 * s, slab.w, 92.0 * s);
        crate::render::shapes::vertical_gradient(
            Rect::new(band.x, band.y, band.w, band.h / 2.0),
            Color::new(0.0, 0.0, 0.0, 0.0),
            Color::new(0.0, 0.0, 0.0, 0.55 * alpha),
        );
        crate::render::shapes::vertical_gradient(
            Rect::new(band.x, band.center().y, band.w, band.h / 2.0),
            Color::new(0.0, 0.0, 0.0, 0.55 * alpha),
            Color::new(0.0, 0.0, 0.0, 0.0),
        );
        if let Some(region) = self.region {
            text::draw_aligned(
                sprites,
                &region.to_uppercase(),
                vec2(band.center().x, band.y + 22.0 * s),
                Align::Center,
                Face::Display,
                16.0 * s,
                faded(TEXT_DIM, alpha),
            );
        }
        text::draw_title(
            sprites,
            &self.level.display_name,
            vec2(band.center().x, band.center().y + 8.0 * s),
            44.0 * s,
            faded(TEXT, alpha),
            faded(INK, alpha),
        );
    }
    pub(super) fn draw_outcome_card(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let s = layout.s;
        let won = self.outcome() == Some(PlayState::Won);
        let since = self.time - self.ended.unwrap_or(self.time) - OUTCOME_DELAY;
        let appear = crate::render::fx::ease_out_back((since / 0.35).min(1.0));
        let card = self.outcome_card_rect(layout);
        let fade = (since / 0.2).min(1.0);
        draw_rectangle(
            layout.main.x,
            layout.main.y,
            layout.main.w,
            layout.main.h,
            Color::new(0.02, 0.01, 0.04, 0.45 * fade),
        );
        let card = Rect::new(card.x, card.y + (1.0 - appear) * 40.0 * s, card.w, card.h);
        ui::panel(card, s);
        let (title, subtitle, color) = if won {
            ("CLEARED!", "Every rat is gone.", GOLD)
        } else {
            let (title, subtitle) = match self.death.expect("a lost level has a death") {
                Death::Bitten => ("BITTEN!", "The rats got you."),
                Death::Blasted => ("KABOOM!", "Caught in the blast."),
                Death::Swallowed => ("SWALLOWED!", "The black hole pulled you in."),
                Death::FriendlyFire => ("FRIENDLY FIRE!", "One hero got the other."),
            };
            (title, subtitle, DANGER)
        };
        if won {
            sprites.additive(|| {
                sprites.draw_at(
                    SpriteId::Glow,
                    vec2(card.center().x, card.y + 52.0 * s),
                    260.0 * s,
                    Color::new(1.0, 0.8, 0.3, 0.35),
                )
            });
        }
        text::draw_title(
            sprites,
            title,
            vec2(card.center().x, card.y + 52.0 * s),
            50.0 * s,
            color,
            INK,
        );
        text::draw_aligned(
            sprites,
            subtitle,
            vec2(card.center().x, card.y + 98.0 * s),
            Align::Center,
            Face::Body,
            21.0 * s,
            TEXT,
        );
        if won {
            let moves = self.game.state.history.len() - 1;
            text::draw_aligned(
                sprites,
                &format!("{moves} moves"),
                vec2(card.center().x, card.y + 126.0 * s),
                Align::Center,
                Face::Display,
                17.0 * s,
                TEXT_DIM,
            );
        }
        for button in self.outcome_buttons(ctx, layout) {
            button.0.draw(sprites, s);
        }
    }
    pub(super) fn outcome_card_rect(&self, layout: &ScreenLayout) -> Rect {
        let s = layout.s;
        let w = (360.0 * s).min(layout.main.w - 20.0 * s);
        let h = 250.0 * s;
        Rect::new(
            layout.main.center().x - w / 2.0,
            layout.main.center().y - h / 2.0,
            w,
            h,
        )
    }
    pub(super) fn outcome_buttons(
        &self,
        ctx: &Ctx,
        layout: &ScreenLayout,
    ) -> Vec<(Button<'static>, UiAction)> {
        let s = layout.s;
        let card = self.outcome_card_rect(layout);
        let won = self.outcome() == Some(PlayState::Won);
        let bw = (card.w - 3.0 * 16.0 * s) / 2.0;
        let bh = 50.0 * s;
        let y = card.bottom() - bh - 46.0 * s;
        let left = Rect::new(card.x + 16.0 * s, y, bw, bh);
        let right = Rect::new(left.right() + 16.0 * s, y, bw, bh);
        let glyphs = action_glyphs(ctx.hints);
        let mut buttons = if won {
            vec![
                (
                    Button {
                        rect: left,
                        icon: Some(SpriteId::IconRestart),
                        label: "Replay",
                        glyph: glyphs.restart,
                        kind: ButtonKind::Normal,
                        enabled: true,
                        focused: false,
                    },
                    UiAction::Restart,
                ),
                (
                    Button {
                        rect: right,
                        icon: Some(SpriteId::IconPlay),
                        label: "Continue",
                        glyph: glyphs.confirm,
                        kind: ButtonKind::Primary,
                        enabled: true,
                        focused: true,
                    },
                    UiAction::Continue,
                ),
            ]
        } else {
            vec![
                (
                    Button {
                        rect: left,
                        icon: Some(SpriteId::IconRestart),
                        label: "Restart",
                        glyph: glyphs.restart,
                        kind: ButtonKind::Normal,
                        enabled: true,
                        focused: false,
                    },
                    UiAction::Restart,
                ),
                (
                    Button {
                        rect: right,
                        icon: Some(SpriteId::IconUndo),
                        label: "Undo",
                        glyph: glyphs.undo,
                        kind: ButtonKind::Primary,
                        enabled: true,
                        focused: true,
                    },
                    UiAction::Undo,
                ),
            ]
        };
        if won {
            buttons.push((
                Button {
                    rect: Rect::new(
                        card.center().x - 110.0 * s,
                        card.bottom() - 40.0 * s,
                        220.0 * s,
                        30.0 * s,
                    ),
                    icon: None,
                    label: "Show solution code",
                    glyph: None,
                    kind: ButtonKind::Normal,
                    enabled: true,
                    focused: false,
                },
                UiAction::Solution,
            ));
        }
        buttons
    }
    pub(super) fn hud(&self, ctx: &Ctx, layout: &ScreenLayout) -> Hud {
        let mut buttons = Vec::new();
        if self.card_visible() {
            buttons.extend(
                self.outcome_buttons(ctx, layout)
                    .into_iter()
                    .map(|(b, action)| (b.rect, action)),
            );
        }
        let rows = control_rows(ctx.hints, self.player_count());
        if layout.portrait() {
            let bar = portrait_bar(layout);
            for (rect, action) in bar {
                if self.action_enabled(action) {
                    buttons.push((rect, action));
                }
            }
            if let Some(top) = layout.top {
                let s = layout.s;
                buttons.push((menu_button_rect(top, s), UiAction::Menu));
            }
        } else {
            for (row, rect) in rows.iter().zip(sidebar_row_rects(layout, rows.len())) {
                if let Some(action) = row.action
                    && self.action_enabled(action)
                {
                    buttons.push((rect, action));
                }
            }
        }
        Hud { buttons, rows }
    }
}

/// What the HUD offers this frame.
pub(super) struct Hud {
    /// Clickable areas.
    buttons: Vec<(Rect, UiAction)>,
    rows: Vec<ControlRow>,
}
/// One line of the controls reference; clickable if it has an action.
struct ControlRow {
    icon: SpriteId,
    label: &'static str,
    glyphs: Vec<Glyph>,
    action: Option<UiAction>,
}
/// Bindings for the actions the HUD buttons stand for.
struct ActionGlyphs {
    undo: Option<Glyph>,
    restart: Option<Glyph>,
    confirm: Option<Glyph>,
}
fn action_glyphs(hints: InputHints) -> ActionGlyphs {
    match hints {
        InputHints::Keyboard => ActionGlyphs {
            undo: Some(Glyph::Key("U")),
            restart: Some(Glyph::Key("R")),
            confirm: Some(Glyph::Key("Space")),
        },
        InputHints::Touch => ActionGlyphs {
            undo: None,
            restart: None,
            confirm: None,
        },
        InputHints::Controller(ty) => ActionGlyphs {
            undo: Some(Glyph::Pad(ty, PadButton::West)),
            restart: Some(Glyph::Pad(ty, PadButton::LeftShoulder)),
            confirm: Some(Glyph::Pad(ty, PadButton::South)),
        },
    }
}
fn control_rows(hints: InputHints, players: usize) -> Vec<ControlRow> {
    let row = |icon, label, glyphs: Vec<Glyph>, action| ControlRow {
        icon,
        label,
        glyphs,
        action,
    };
    let two = players > 1;
    let mut rows = Vec::new();
    match hints {
        InputHints::Keyboard | InputHints::Touch if !two => {
            rows.push(row(
                SpriteId::IconPlay,
                "Move",
                vec![Glyph::Arrows, Glyph::Wasd],
                None,
            ));
            rows.push(row(
                SpriteId::IconWait,
                "Wait",
                vec![Glyph::Key("Space")],
                Some(UiAction::Wait),
            ));
            rows.push(row(
                SpriteId::IconUndo,
                "Undo",
                vec![Glyph::Key("U")],
                Some(UiAction::Undo),
            ));
            rows.push(row(
                SpriteId::IconRestart,
                "Restart",
                vec![Glyph::Key("R")],
                Some(UiAction::Restart),
            ));
            rows.push(row(
                SpriteId::IconMenu,
                "Menu",
                vec![Glyph::Key("Esc")],
                Some(UiAction::Menu),
            ));
        }
        InputHints::Keyboard | InputHints::Touch => {
            rows.push(row(
                SpriteId::IconPlayers,
                "P1 move",
                vec![Glyph::Arrows],
                None,
            ));
            rows.push(row(
                SpriteId::IconPlayers,
                "P2 move",
                vec![Glyph::Wasd],
                None,
            ));
            rows.push(row(
                SpriteId::IconStar,
                "Sync move",
                vec![Glyph::Key("Shift")],
                None,
            ));
            rows.push(row(
                SpriteId::IconWait,
                "Both wait",
                vec![Glyph::Key("Space")],
                Some(UiAction::Wait),
            ));
            rows.push(row(
                SpriteId::IconUndo,
                "Undo",
                vec![Glyph::Key("U")],
                Some(UiAction::Undo),
            ));
            rows.push(row(
                SpriteId::IconRestart,
                "Restart",
                vec![Glyph::Key("R")],
                Some(UiAction::Restart),
            ));
            rows.push(row(
                SpriteId::IconMenu,
                "Menu",
                vec![Glyph::Key("Esc")],
                Some(UiAction::Menu),
            ));
        }
        InputHints::Controller(ty) => {
            let pad = |b| Glyph::Pad(ty, b);
            if two {
                rows.push(row(
                    SpriteId::IconPlayers,
                    "P1 move",
                    vec![pad(PadButton::DPad)],
                    None,
                ));
                rows.push(row(
                    SpriteId::IconPlayers,
                    "P2 move",
                    vec![Glyph::Arrows],
                    None,
                ));
                rows.push(row(
                    SpriteId::IconStar,
                    "Sync move",
                    vec![pad(PadButton::RightShoulder)],
                    None,
                ));
            } else {
                rows.push(row(
                    SpriteId::IconPlay,
                    "Move",
                    vec![pad(PadButton::DPad), pad(PadButton::Stick)],
                    None,
                ));
            }
            rows.push(row(
                SpriteId::IconWait,
                "Wait",
                vec![pad(PadButton::South)],
                Some(UiAction::Wait),
            ));
            rows.push(row(
                SpriteId::IconUndo,
                "Undo",
                vec![pad(PadButton::West)],
                Some(UiAction::Undo),
            ));
            rows.push(row(
                SpriteId::IconRestart,
                "Restart",
                vec![pad(PadButton::LeftShoulder)],
                Some(UiAction::Restart),
            ));
            rows.push(row(
                SpriteId::IconMenu,
                "Menu",
                vec![pad(PadButton::Start)],
                Some(UiAction::Menu),
            ));
        }
    }
    if hints != InputHints::Touch {
        rows.push(row(SpriteId::IconDrag, "Plan a path", vec![], None));
    }
    rows
}
const ROW_H: f32 = 40.0;
/// Where the controls rows go in the sidebar (bottom-aligned).
fn sidebar_row_rects(layout: &ScreenLayout, count: usize) -> Vec<Rect> {
    let s = layout.s;
    let p = layout.panel;
    let pad = 14.0 * s;
    let h = ROW_H * s;
    let top = p.bottom() - pad - h * count as f32;
    (0..count)
        .map(|i| Rect::new(p.x + pad, top + i as f32 * h, p.w - 2.0 * pad, h - 4.0 * s))
        .collect()
}
fn rats_left(scene: &LevelScene) -> usize {
    scene
        .game
        .state
        .grid
        .entries()
        .filter(|(_, c)| matches!(c, Cell::Rat(_) | Cell::CyborgRat(_)))
        .count()
}
fn draw_sidebar(
    sprites: &Sprites,
    scene: &LevelScene,
    ctx: &Ctx,
    layout: &ScreenLayout,
    rows: &[ControlRow],
) {
    let s = layout.s;
    let p = layout.panel;
    ui::panel(p, s);
    let pad = 18.0 * s;
    let x = p.x + pad;
    let w = p.w - 2.0 * pad;
    let mut y = p.y + pad + 8.0 * s;

    if let Some(region) = scene.region {
        ui::heading(sprites, &region.to_uppercase(), vec2(x, y), s);
        y += 30.0 * s;
    }
    let title_size = fit_size(
        sprites,
        &scene.level.display_name,
        Face::Display,
        34.0 * s,
        w,
    );
    text::draw_aligned(
        sprites,
        &scene.level.display_name,
        vec2(x, y),
        Align::Left,
        Face::Display,
        title_size,
        TEXT,
    );
    y += 34.0 * s;
    draw_status_chips(sprites, scene, ctx, vec2(x, y), s);
    y += 34.0 * s;

    let rows_top = sidebar_row_rects(layout, rows.len())
        .first()
        .map_or(p.bottom(), |r| r.y);
    let controls_heading = rows_top - 26.0 * s;
    let space = Rect::new(x, y, w, controls_heading - y - 12.0 * s);
    let used = draw_dialogue(sprites, scene, space, s, false);
    y += used + if used > 0.0 { 18.0 * s } else { 0.0 };
    draw_field_guide(
        sprites,
        scene,
        Rect::new(x, y, w, controls_heading - y - 10.0 * s),
        s,
    );

    ui::heading(sprites, "CONTROLS", vec2(x, controls_heading + 8.0 * s), s);
    for (row, rect) in rows.iter().zip(sidebar_row_rects(layout, rows.len())) {
        draw_control_row(sprites, scene, row, rect, s);
    }
}
/// The largest font size up to `max` at which `text` fits in `width`.
fn fit_size(sprites: &Sprites, label: &str, face: Face, max: f32, width: f32) -> f32 {
    let w = text::width(sprites, label, face, max);
    if w <= width { max } else { max * width / w }
}
fn draw_status_chips(sprites: &Sprites, scene: &LevelScene, ctx: &Ctx, pos: Vec2, s: f32) {
    let rats = rats_left(scene);
    let h = 28.0 * s;
    let mut x = pos.x;
    let chip = |x: f32, icon: SpriteId, label: &str, color: Color| -> f32 {
        let w = text::width(sprites, label, Face::Display, 17.0 * s) + h + 14.0 * s;
        let r = Rect::new(x, pos.y - h / 2.0, w, h);
        rounded_rect(r, h / 2.0, Color::new(0.0, 0.0, 0.0, 0.3));
        sprites.draw_at(icon, vec2(x + h * 0.55, pos.y), h * 0.62, color);
        text::draw_aligned(
            sprites,
            label,
            vec2(x + h, pos.y),
            Align::Left,
            Face::Display,
            17.0 * s,
            color,
        );
        w
    };
    x += chip(
        x,
        SpriteId::IconRat,
        &format!("{rats} left"),
        if rats == 0 { GOLD } else { TEXT },
    ) + 8.0 * s;
    let moves = scene.game.state.history.len() - 1;
    let moves_label = if moves == 1 {
        "1 move".to_string()
    } else {
        format!("{moves} moves")
    };
    x += chip(x, SpriteId::IconSword, &moves_label, TEXT_DIM) + 8.0 * s;
    if ctx.progress.is_completed(scene.level.name) {
        chip(x, SpriteId::IconCheck, "Cleared", palette::SUCCESS);
    }
}
/// What a thing in the field guide looks like.
#[derive(Clone, Copy)]
enum GuideIcon {
    Sprite(SpriteId),
    Trigger,
}
/// A kind of thing in the field guide.
struct GuideEntry {
    present: fn(Cell) -> bool,
    icon: GuideIcon,
    name: &'static str,
    rule: &'static str,
}
/// The special things in a level, with a reminder of how each behaves,
/// most intricate first.
fn field_guide(level: &Level) -> Vec<&'static GuideEntry> {
    const ENTRIES: [GuideEntry; 7] = [
        GuideEntry {
            present: |c| matches!(c, Cell::Trigger(_)),
            icon: GuideIcon::Trigger,
            name: "Trigger",
            rule: "Stepping on one turns the others with its number into walls, along with the empty cells next to them.",
        },
        GuideEntry {
            present: |c| c == Cell::Explosive,
            icon: GuideIcon::Sprite(SpriteId::Keg),
            name: "Powder keg",
            rule: "Blasts everything next to it when stepped on, zapped, or caught in a blast.",
        },
        GuideEntry {
            present: |c| c == Cell::BlackHole,
            icon: GuideIcon::Sprite(SpriteId::HoleBase),
            name: "Black hole",
            rule: "Swallows whatever steps in.",
        },
        GuideEntry {
            present: |c| matches!(c, Cell::CyborgRat(_)),
            icon: GuideIcon::Sprite(SpriteId::Cyborg0),
            name: "Cyborg rat",
            rule: "Hunts you around walls, trampling rats on the way.",
        },
        GuideEntry {
            present: |c| c == Cell::Spiderweb,
            icon: GuideIcon::Sprite(SpriteId::Web),
            name: "Web",
            rule: "Rats can't get through. Your sword cuts it.",
        },
        GuideEntry {
            present: |c| c == Cell::Plank,
            icon: GuideIcon::Sprite(SpriteId::Plank),
            name: "Planks",
            rule: "Block you, but rats break through.",
        },
        GuideEntry {
            present: |c| matches!(c, Cell::Rat(_)),
            icon: GuideIcon::Sprite(SpriteId::Rat0),
            name: "Rat",
            rule: "Chases you, but can't bite past your sword.",
        },
    ];
    ENTRIES
        .iter()
        .filter(|entry| level.grid.entries().any(|(_, c)| (entry.present)(c)))
        .collect()
}
/// A legend of what's in the room, as many entries as fit in `space`.
fn draw_field_guide(sprites: &Sprites, scene: &LevelScene, space: Rect, s: f32) {
    let entries = field_guide(scene.level);
    if entries.is_empty() || space.h < 60.0 * s {
        return;
    }
    ui::heading(sprites, "IN THIS ROOM", vec2(space.x, space.y + 8.0 * s), s);
    let mut y = space.y + 26.0 * s;
    let icon = 34.0 * s;
    let text_x = space.x + icon + 12.0 * s;
    let size = 16.0 * s;
    for entry in entries {
        let lines = text::wrap(
            sprites,
            entry.rule,
            Face::Body,
            size,
            space.right() - text_x,
        );
        let h = 20.0 * s + lines.len() as f32 * size * 1.15 + 8.0 * s;
        if y + h > space.bottom() {
            break;
        }
        let center = vec2(space.x + icon / 2.0, y + icon / 2.0 + 2.0 * s);
        match entry.icon {
            GuideIcon::Sprite(id) => sprites.draw_at(id, center, icon * board::SPRITE_SPAN, WHITE),
            GuideIcon::Trigger => board::draw_trigger(sprites, center, icon, 1, scene.time),
        }
        text::draw_aligned(
            sprites,
            entry.name,
            vec2(text_x, y + 9.0 * s),
            Align::Left,
            Face::Display,
            17.0 * s,
            TEXT,
        );
        let mut ly = y + 9.0 * s + 19.0 * s;
        for line in lines {
            text::draw_aligned(
                sprites,
                &line,
                vec2(text_x, ly),
                Align::Left,
                Face::Body,
                size,
                TEXT_DIM,
            );
            ly += size * 1.15;
        }
        y += h;
    }
}
/// The note being read, typed out beside the speaker's portrait. Returns
/// the height used.
fn draw_dialogue(sprites: &Sprites, scene: &LevelScene, space: Rect, s: f32, compact: bool) -> f32 {
    let d = &scene.dialogue;
    let Some(player) = d.player else {
        return 0.0;
    };
    let portrait = (if compact { 56.0 } else { 72.0 }) * s;
    let size = (if compact { 18.0 } else { 20.0 }) * s;
    let line_h = size * 1.22;
    let text_x = space.x + portrait + 14.0 * s;
    let text_w = space.right() - text_x - 8.0 * s;
    let lines = text::wrap(sprites, &d.text, Face::Body, size, text_w);
    let height = (lines.len() as f32 * line_h + 22.0 * s).max(portrait + 16.0 * s);
    let card = Rect::new(
        space.x,
        space.y,
        space.w,
        height.min(space.h.max(portrait + 16.0 * s)),
    );
    ui::card(card, s);
    let portrait_sprite = match player {
        Player::Player1 => SpriteId::PortraitHero1,
        Player::Player2 => SpriteId::PortraitHero2,
    };
    let pc = vec2(
        space.x + 8.0 * s + portrait / 2.0,
        card.y + 8.0 * s + portrait / 2.0,
    );
    crate::render::shapes::circle(pc, portrait * 0.5, Color::new(0.0, 0.0, 0.0, 0.3));
    sprites.draw_at(portrait_sprite, pc, portrait, WHITE);
    let mut remaining = d.shown as usize;
    let mut y = card.y + 12.0 * s + size * 0.6;
    for line in lines {
        let n = line.chars().count();
        let visible: String = line.chars().take(remaining).collect();
        remaining = remaining.saturating_sub(n + 1);
        text::draw_aligned(
            sprites,
            &visible,
            vec2(text_x, y),
            Align::Left,
            Face::Body,
            size,
            TEXT,
        );
        y += line_h;
    }
    card.h
}
fn draw_control_row(sprites: &Sprites, scene: &LevelScene, row: &ControlRow, rect: Rect, s: f32) {
    let clickable = row.action.is_some();
    let enabled = row.action.is_none_or(|a| scene.action_enabled(a));
    let armed = row.action == Some(UiAction::Restart) && scene.restart_pending();
    if clickable && enabled && ui::hovered(rect) || armed {
        rounded_rect(
            rect,
            9.0 * s,
            Color::new(1.0, 1.0, 1.0, if armed { 0.14 } else { 0.07 }),
        );
    }
    let color = if enabled { TEXT } else { faded(TEXT_DIM, 0.5) };
    let cy = rect.center().y;
    sprites.draw_at(
        row.icon,
        vec2(rect.x + 16.0 * s, cy),
        22.0 * s,
        if clickable { GOLD } else { TEXT_DIM },
    );
    text::draw_aligned(
        sprites,
        if armed { "Again!" } else { row.label },
        vec2(rect.x + 38.0 * s, cy),
        Align::Left,
        Face::Display,
        18.0 * s,
        color,
    );
    if row.glyphs.is_empty() {
        text::draw_aligned(
            sprites,
            "drag from hero",
            vec2(rect.right() - 6.0 * s, cy),
            Align::Right,
            Face::Body,
            16.0 * s,
            TEXT_DIM,
        );
    }
    let glyph_height = |g: Glyph| match g {
        Glyph::Arrows | Glyph::Wasd => 33.0 * s,
        _ => 26.0 * s,
    };
    let total: f32 = row
        .glyphs
        .iter()
        .map(|&g| crate::render::glyphs::width(sprites, g, glyph_height(g)) + 6.0 * s)
        .sum();
    let mut gx = rect.right() - 6.0 * s - total + 6.0 * s;
    for &g in &row.glyphs {
        gx += crate::render::glyphs::draw(sprites, g, vec2(gx, cy), glyph_height(g)) + 6.0 * s;
    }
}
fn menu_button_rect(top: Rect, s: f32) -> Rect {
    Rect::new(
        top.x + 6.0 * s,
        top.y + 6.0 * s,
        top.h - 12.0 * s,
        top.h - 12.0 * s,
    )
}
/// The portrait-mode button bar: Undo, Wait, Restart, Menu.
fn portrait_bar(layout: &ScreenLayout) -> Vec<(Rect, UiAction)> {
    let s = layout.s;
    let p = layout.panel;
    let pad = 10.0 * s;
    let h = 58.0 * s;
    let y = p.bottom() - pad - h;
    let actions = [UiAction::Undo, UiAction::Wait, UiAction::Restart];
    let w = (p.w - pad * (actions.len() as f32 + 1.0)) / actions.len() as f32;
    actions
        .iter()
        .enumerate()
        .map(|(i, &a)| (Rect::new(p.x + pad + i as f32 * (w + pad), y, w, h), a))
        .collect()
}
fn draw_portrait_hud(sprites: &Sprites, scene: &LevelScene, ctx: &Ctx, layout: &ScreenLayout) {
    let s = layout.s;
    if let Some(top) = layout.top {
        ui::panel(top, s);
        let menu = menu_button_rect(top, s);
        Button {
            rect: menu,
            icon: Some(SpriteId::IconMenu),
            label: "",
            glyph: None,
            kind: ButtonKind::Normal,
            enabled: true,
            focused: false,
        }
        .draw(sprites, s);
        let x = menu.right() + 12.0 * s;
        let rats = rats_left(scene);
        let chip_w = 70.0 * s;
        let name_w = top.right() - x - chip_w - 12.0 * s;
        let size = fit_size(
            sprites,
            &scene.level.display_name,
            Face::Display,
            24.0 * s,
            name_w,
        );
        text::draw_aligned(
            sprites,
            &scene.level.display_name,
            vec2(
                x,
                top.center().y + if scene.region.is_some() { 7.0 * s } else { 0.0 },
            ),
            Align::Left,
            Face::Display,
            size,
            TEXT,
        );
        if let Some(region) = scene.region {
            text::draw_aligned(
                sprites,
                &region.to_uppercase(),
                vec2(x, top.y + 13.0 * s),
                Align::Left,
                Face::Display,
                11.0 * s,
                TEXT_DIM,
            );
        }
        let cx = top.right() - chip_w / 2.0 - 8.0 * s;
        sprites.draw_at(
            SpriteId::IconRat,
            vec2(cx - 14.0 * s, top.center().y),
            24.0 * s,
            if rats == 0 { GOLD } else { TEXT },
        );
        text::draw_aligned(
            sprites,
            &format!("×{rats}"),
            vec2(cx + 2.0 * s, top.center().y),
            Align::Left,
            Face::Display,
            20.0 * s,
            TEXT,
        );
    }
    ui::panel(layout.panel, s);
    let bar = portrait_bar(layout);
    let bar_top = bar.first().map_or(layout.panel.bottom(), |(r, _)| r.y);
    let pad = 10.0 * s;
    let space = Rect::new(
        layout.panel.x + pad,
        layout.panel.y + pad,
        layout.panel.w - 2.0 * pad,
        bar_top - layout.panel.y - 2.0 * pad,
    );
    if scene.dialogue.player.is_some() {
        draw_dialogue(sprites, scene, space, s, true);
    } else {
        let hint = match ctx.hints {
            InputHints::Touch => "Swipe to move. Drag from your hero to plan a path.",
            InputHints::Keyboard => "Arrows or WASD to move. Space waits a turn.",
            InputHints::Controller(_) => "D-pad or stick to move.",
        };
        let lines = text::wrap(sprites, hint, Face::Body, 17.0 * s, space.w);
        let mut y = space.center().y - (lines.len() as f32 - 1.0) * 11.0 * s;
        for line in lines {
            text::draw_aligned(
                sprites,
                &line,
                vec2(space.center().x, y),
                Align::Center,
                Face::Body,
                17.0 * s,
                TEXT_DIM,
            );
            y += 22.0 * s;
        }
    }
    let glyphs = action_glyphs(ctx.hints);
    let armed = scene.restart_pending();
    for (rect, action) in bar {
        let (icon, label, glyph) = match action {
            UiAction::Undo => (SpriteId::IconUndo, "Undo", glyphs.undo),
            UiAction::Wait => (SpriteId::IconWait, "Wait", None),
            UiAction::Restart => (
                SpriteId::IconRestart,
                if armed { "Again!" } else { "Restart" },
                glyphs.restart,
            ),
            _ => unreachable!("not on the portrait bar"),
        };
        let enabled = scene.action_enabled(action);
        Button {
            rect,
            icon: Some(icon),
            label,
            glyph: if rect.w > 150.0 * s { glyph } else { None },
            kind: ButtonKind::Normal,
            enabled,
            focused: armed && action == UiAction::Restart,
        }
        .draw(sprites, s);
    }
}

impl Hud {
    pub(super) fn hit(&self, pos: Vec2) -> Option<UiAction> {
        // Later buttons (cards) sit on top.
        self.buttons
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(pos))
            .map(|&(_, action)| action)
    }

    pub(super) fn draw(
        &self,
        sprites: &Sprites,
        scene: &LevelScene,
        ctx: &Ctx,
        layout: &ScreenLayout,
    ) {
        if layout.portrait() {
            draw_portrait_hud(sprites, scene, ctx, layout);
        } else {
            draw_sidebar(sprites, scene, ctx, layout, &self.rows);
        }
    }
}
