//! Modal overlays: the pause menu and the copyable-code dialog.

use macroquad::prelude::*;

use crate::atlas::SpriteId;
use crate::audio::Sfx;
use crate::direction::Dir4;
use crate::input::MetaInput;
use crate::render::palette::{GOLD, TEXT, TEXT_DIM, rgb};
use crate::render::shapes::{faded, rounded_rect, rounded_rect_outline};
use crate::render::text::{self, Align};
use crate::render::ui::{self, Button, ButtonKind, ScreenLayout};
use crate::scenes::{Ctx, FrameInput};
use crate::settings::Settings;
use crate::sprites::Face;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MenuItem {
    Resume,
    Restart,
    WorldMap,
    Music,
    Sound,
    Shake,
    Export,
    Import,
    Title,
    Quit,
}

/// What the app should do after a menu choice.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum MenuChoice {
    Resume,
    Restart,
    WorldMap,
    Export,
    Import,
    Title,
    Quit,
}

pub(crate) struct Menu {
    items: Vec<MenuItem>,
    focus: usize,
    time: f32,
}

impl Menu {
    /// The pause menu for a level (`in_level`) or the world map.
    pub(crate) fn new(in_level: bool) -> Self {
        let mut items = vec![MenuItem::Resume];
        if in_level {
            items.extend([MenuItem::Restart, MenuItem::WorldMap]);
        }
        items.extend([
            MenuItem::Music,
            MenuItem::Sound,
            MenuItem::Shake,
            MenuItem::Export,
            MenuItem::Import,
        ]);
        if !in_level {
            items.push(MenuItem::Title);
        }
        if cfg!(not(target_arch = "wasm32")) {
            items.push(MenuItem::Quit);
        }
        Self {
            items,
            focus: 0,
            time: 0.0,
        }
    }

    fn rows(&self, layout: &ScreenLayout) -> (Rect, Vec<Rect>) {
        let s = layout.s;
        let row_h = 52.0 * s;
        let gap = 8.0 * s;
        let w = (420.0 * s).min(screen_width() - 24.0 * s);
        let h = 96.0 * s + self.items.len() as f32 * (row_h + gap) + 14.0 * s;
        let card = Rect::new(
            screen_width() / 2.0 - w / 2.0,
            (screen_height() / 2.0 - h / 2.0).max(8.0 * s),
            w,
            h,
        );
        let rows = (0..self.items.len())
            .map(|i| {
                Rect::new(
                    card.x + 20.0 * s,
                    card.y + 86.0 * s + i as f32 * (row_h + gap),
                    card.w - 40.0 * s,
                    row_h,
                )
            })
            .collect();
        (card, rows)
    }

    fn adjust(&self, item: MenuItem, delta: f32, ctx: &mut Ctx) {
        let value = match item {
            MenuItem::Music => &mut ctx.settings.music_volume,
            MenuItem::Sound => &mut ctx.settings.sfx_volume,
            MenuItem::Shake => &mut ctx.settings.screen_shake,
            _ => return,
        };
        *value = ((*value + delta) * 10.0).round().clamp(0.0, 10.0) / 10.0;
        ctx.audio.music_volume = ctx.settings.music_volume;
        ctx.audio.sfx_volume = ctx.settings.sfx_volume;
        ctx.settings.save();
        ctx.audio.play(Sfx::UiMove);
    }

    fn choose(&self, item: MenuItem, ctx: &mut Ctx) -> Option<MenuChoice> {
        let choice = match item {
            MenuItem::Resume => MenuChoice::Resume,
            MenuItem::Restart => MenuChoice::Restart,
            MenuItem::WorldMap => MenuChoice::WorldMap,
            MenuItem::Export => MenuChoice::Export,
            MenuItem::Import => MenuChoice::Import,
            MenuItem::Title => MenuChoice::Title,
            MenuItem::Quit => MenuChoice::Quit,
            MenuItem::Music | MenuItem::Sound | MenuItem::Shake => {
                let on = slider_value(item, ctx.settings).expect("a slider") > 0.0;
                self.adjust(item, if on { -1.0 } else { 1.0 }, ctx);
                return None;
            }
        };
        ctx.audio.play(if choice == MenuChoice::Resume {
            Sfx::UiBack
        } else {
            Sfx::UiConfirm
        });
        Some(choice)
    }

    pub(crate) fn update(
        &mut self,
        ctx: &mut Ctx,
        input: &FrameInput,
        layout: &ScreenLayout,
    ) -> Option<MenuChoice> {
        self.time += input.dt;
        match input.nav {
            Some(Dir4::North) => {
                self.focus = (self.focus + self.items.len() - 1) % self.items.len();
                ctx.audio.play(Sfx::UiMove);
            }
            Some(Dir4::South) => {
                self.focus = (self.focus + 1) % self.items.len();
                ctx.audio.play(Sfx::UiMove);
            }
            Some(Dir4::West) => self.adjust(self.items[self.focus], -0.1, ctx),
            Some(Dir4::East) => self.adjust(self.items[self.focus], 0.1, ctx),
            None => {}
        }
        if input.has(MetaInput::Exit) || input.has(MetaInput::Undo) {
            ctx.audio.play(Sfx::UiBack);
            return Some(MenuChoice::Resume);
        }
        if input.confirmed() {
            return self.choose(self.items[self.focus], ctx);
        }
        let (card, rows) = self.rows(layout);
        for click in input.clicks() {
            if !card.contains(click) {
                ctx.audio.play(Sfx::UiBack);
                return Some(MenuChoice::Resume);
            }
            for (i, row) in rows.iter().enumerate() {
                if !row.contains(click) {
                    continue;
                }
                self.focus = i;
                let item = self.items[i];
                if let Some(current) = slider_value(item, ctx.settings) {
                    let slider = slider_rect(*row, layout.s);
                    let value = ((click.x - slider.x) / slider.w).clamp(0.0, 1.0);
                    self.adjust(item, value - current, ctx);
                    return None;
                }
                return self.choose(item, ctx);
            }
        }
        None
    }

    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let s = layout.s;
        let appear = (self.time / 0.18).min(1.0);
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.02, 0.01, 0.04, 0.6 * appear),
        );
        let (card, rows) = self.rows(layout);
        let card = Rect::new(card.x, card.y + (1.0 - appear) * 24.0 * s, card.w, card.h);
        ui::panel(card, s);
        text::draw_title(
            sprites,
            "Paused",
            vec2(card.center().x, card.y + 46.0 * s),
            40.0 * s,
            GOLD,
            crate::render::palette::INK,
        );
        for (i, (item, row)) in self.items.iter().zip(rows).enumerate() {
            let row = Rect::new(row.x, row.y + (1.0 - appear) * 24.0 * s, row.w, row.h);
            let focused = i == self.focus;
            let (icon, label) = match item {
                MenuItem::Resume => (SpriteId::IconPlay, "Resume"),
                MenuItem::Restart => (SpriteId::IconRestart, "Restart level"),
                MenuItem::WorldMap => (SpriteId::IconMap, "World map"),
                MenuItem::Music => (SpriteId::IconMusic, "Music"),
                MenuItem::Sound => (SpriteId::IconSound, "Sound"),
                MenuItem::Shake => (SpriteId::IconGear, "Screen shake"),
                MenuItem::Export => (SpriteId::IconExport, "Export progress"),
                MenuItem::Import => (SpriteId::IconImport, "Import progress"),
                MenuItem::Title => (SpriteId::IconDoor, "Title screen"),
                MenuItem::Quit => (SpriteId::IconClose, "Quit game"),
            };
            match slider_value(*item, ctx.settings) {
                Some(value) => {
                    if focused || ui::hovered(row) {
                        rounded_rect(row, 12.0 * s, faded(rgb(0x3a3249), 0.9));
                    }
                    if focused {
                        rounded_rect_outline(row, 12.0 * s, 2.0 * s, GOLD);
                    }
                    let icon = if value == 0.0 && *item != MenuItem::Shake {
                        SpriteId::IconMute
                    } else {
                        icon
                    };
                    sprites.draw_at(
                        icon,
                        vec2(row.x + 26.0 * s, row.center().y),
                        26.0 * s,
                        if focused { GOLD } else { TEXT },
                    );
                    text::draw_aligned(
                        sprites,
                        label,
                        vec2(row.x + 50.0 * s, row.center().y),
                        Align::Left,
                        Face::Display,
                        21.0 * s,
                        TEXT,
                    );
                    ui::slider(slider_rect(row, s), value, focused, s);
                    text::draw_aligned(
                        sprites,
                        &format!("{}", (value * 10.0).round() as i32),
                        vec2(row.right() - 14.0 * s, row.center().y),
                        Align::Right,
                        Face::Display,
                        18.0 * s,
                        TEXT_DIM,
                    );
                }
                _ => Button {
                    rect: row,
                    icon: Some(icon),
                    label,
                    glyph: None,
                    kind: if *item == MenuItem::Resume {
                        ButtonKind::Primary
                    } else {
                        ButtonKind::Normal
                    },
                    enabled: true,
                    focused,
                }
                .draw(sprites, s),
            }
        }
    }
}

fn slider_rect(row: Rect, s: f32) -> Rect {
    let x = row.x + row.w * 0.52;
    Rect::new(x, row.y, row.right() - x - 46.0 * s, row.h)
}

/// A dialog showing a code to copy (progress export or a solution).
pub(crate) struct CodeDialog {
    pub(crate) title: &'static str,
    pub(crate) code: String,
    copied: bool,
}

impl CodeDialog {
    pub(crate) fn new(title: &'static str, code: String) -> Self {
        Self {
            title,
            code,
            copied: false,
        }
    }

    fn layout(&self, ctx: &Ctx, layout: &ScreenLayout) -> (Rect, Vec<String>, Rect) {
        let s = layout.s;
        let w = (520.0 * s).min(screen_width() - 24.0 * s);
        let size = 15.0 * s;
        let lines = wrap_chars(ctx, &self.code, size, w - 64.0 * s);
        let text_h = lines.len() as f32 * size * 1.35 + 20.0 * s;
        let h = 80.0 * s + text_h + 86.0 * s;
        let card = Rect::new(
            screen_width() / 2.0 - w / 2.0,
            screen_height() / 2.0 - h / 2.0,
            w,
            h,
        );
        let button = Rect::new(
            card.center().x - 90.0 * s,
            card.bottom() - 70.0 * s,
            180.0 * s,
            52.0 * s,
        );
        (card, lines, button)
    }

    /// Returns true when the dialog should close.
    pub(crate) fn update(
        &mut self,
        ctx: &mut Ctx,
        input: &FrameInput,
        layout: &ScreenLayout,
    ) -> bool {
        let (card, _, button) = self.layout(ctx, layout);
        if input.has(MetaInput::Exit) || input.has(MetaInput::Undo) {
            ctx.audio.play(Sfx::UiBack);
            return true;
        }
        let copy = input.confirmed() || input.clicks().any(|t| button.contains(t));
        if copy && !self.copied {
            crate::storage::progress::copy_to_clipboard(&self.code);
            self.copied = true;
            ctx.audio.play(Sfx::UiConfirm);
            return false;
        }
        if copy || input.clicks().any(|t| !card.contains(t)) {
            ctx.audio.play(Sfx::UiBack);
            return true;
        }
        false
    }

    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let s = layout.s;
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.02, 0.01, 0.04, 0.65),
        );
        let (card, lines, button) = self.layout(ctx, layout);
        ui::panel(card, s);
        text::draw_title(
            sprites,
            self.title,
            vec2(card.center().x, card.y + 40.0 * s),
            30.0 * s,
            GOLD,
            crate::render::palette::INK,
        );
        let size = 15.0 * s;
        let area = Rect::new(
            card.x + 20.0 * s,
            card.y + 74.0 * s,
            card.w - 40.0 * s,
            lines.len() as f32 * size * 1.35 + 20.0 * s,
        );
        rounded_rect(area, 10.0 * s, rgb(0x15121c));
        let mut y = area.y + 10.0 * s + size * 0.5;
        for line in &lines {
            text::draw_aligned(
                sprites,
                line,
                vec2(area.x + 12.0 * s, y),
                Align::Left,
                Face::Body,
                size,
                rgb(0xcfc6e0),
            );
            y += size * 1.35;
        }
        Button {
            rect: button,
            icon: Some(if self.copied {
                SpriteId::IconCheck
            } else {
                SpriteId::IconExport
            }),
            label: if self.copied { "Copied!" } else { "Copy" },
            glyph: None,
            kind: ButtonKind::Primary,
            enabled: true,
            focused: true,
        }
        .draw(sprites, s);
    }
}

/// Break a long unspaced string into lines no wider than `max_width`.
fn wrap_chars(ctx: &Ctx, code: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut width = 0.0;
    for ch in code.chars() {
        let advance = text::width(ctx.sprites, ch.encode_utf8(&mut [0; 4]), Face::Body, size);
        if width + advance > max_width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            width = 0.0;
        }
        line.push(ch);
        width += advance;
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The setting a slider row shows, if it's a slider.
fn slider_value(item: MenuItem, settings: &Settings) -> Option<f32> {
    match item {
        MenuItem::Music => Some(settings.music_volume),
        MenuItem::Sound => Some(settings.sfx_volume),
        MenuItem::Shake => Some(settings.screen_shake),
        _ => None,
    }
}
