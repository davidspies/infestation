//! Modal overlays: the pause menu and the copyable-code dialog.

use macroquad::prelude::*;

use crate::atlas::SpriteId;
use crate::audio::Sfx;
use crate::direction::Dir4;
use crate::input::{MetaInput, PointerEvent};
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
    TouchVisibility,
    TouchLayout,
    TouchRepeat,
    Export,
    Import,
    Title,
    Quit,
}

impl MenuItem {
    fn is_slider(self) -> bool {
        matches!(
            self,
            Self::Music | Self::Sound | Self::Shake | Self::TouchRepeat
        )
    }
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

/// One geometry calculation serves painting, hit testing and focus scrolling.
struct MenuLayout {
    s: f32,
    card: Rect,
    viewport: Rect,
    rows: Vec<Rect>,
    scroll: f32,
    max_scroll: f32,
}

impl MenuLayout {
    fn new(width: f32, height: f32, scale: f32, count: usize, time: f32, scroll: f32) -> Self {
        // The game can use a tiny scale beside the landscape dock. Menu rows
        // keep usable text and touch targets instead of inheriting that scale.
        let s = scale.max(0.85);
        let margin = 8.0 * s;
        let row_h = (52.0 * s).max(44.0);
        let gap = 8.0 * s;
        let content_h = count as f32 * (row_h + gap) - gap;
        let w = (420.0 * s).min((width - 2.0 * margin).max(1.0));
        let h = (106.0 * s + content_h).min((height - 2.0 * margin).max(1.0));
        let appear = (time / 0.18).min(1.0);
        let y = (height / 2.0 - h / 2.0 + (1.0 - appear) * 24.0 * s)
            .clamp(margin, (height - h - margin).max(margin));
        let card = Rect::new((width - w) / 2.0, y, w, h);
        let viewport = Rect::new(
            card.x + 20.0 * s,
            card.y + 86.0 * s,
            (card.w - 40.0 * s).max(1.0),
            (card.h - 106.0 * s).max(0.0),
        );
        let max_scroll = (content_h - viewport.h).max(0.0);
        let scroll = scroll.clamp(0.0, max_scroll);
        let row_w = viewport.w - if max_scroll > 0.0 { 12.0 * s } else { 0.0 };
        let rows = (0..count)
            .map(|i| {
                Rect::new(
                    viewport.x,
                    viewport.y + i as f32 * (row_h + gap) - scroll,
                    row_w,
                    row_h,
                )
            })
            .collect();
        Self {
            s,
            card,
            viewport,
            rows,
            scroll,
            max_scroll,
        }
    }

    fn row_at(&self, point: Vec2) -> Option<usize> {
        self.viewport
            .contains(point)
            .then(|| self.rows.iter().position(|row| row.contains(point)))
            .flatten()
    }
}

#[derive(Clone, Copy)]
enum MenuGesture {
    Idle,
    Slider {
        index: usize,
        track: Rect,
    },
    Scroll {
        start: Vec2,
        initial_scroll: f32,
        moved: bool,
    },
    /// Ignore a release belonging to an interrupted gesture.
    Cancelled,
}

#[derive(Debug, PartialEq)]
enum MenuPointerAction {
    SetSlider { index: usize, value: f32 },
    Choose(usize),
    Close,
}

pub(crate) struct Menu {
    items: Vec<MenuItem>,
    focus: usize,
    time: f32,
    scroll: f32,
    gesture: MenuGesture,
}

impl Menu {
    /// The pause menu for a level (`in_level`) or the world map.
    pub(crate) fn new(in_level: bool) -> Self {
        let mut items = vec![MenuItem::Resume];
        if in_level {
            items.extend([MenuItem::Restart, MenuItem::WorldMap]);
        }
        items.extend([MenuItem::Music, MenuItem::Sound, MenuItem::Shake]);
        if cfg!(target_arch = "wasm32") {
            items.extend([
                MenuItem::TouchVisibility,
                MenuItem::TouchLayout,
                MenuItem::TouchRepeat,
            ]);
        }
        items.extend([MenuItem::Export, MenuItem::Import]);
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
            scroll: 0.0,
            gesture: MenuGesture::Cancelled,
        }
    }

    fn layout(&self, layout: &ScreenLayout) -> MenuLayout {
        MenuLayout::new(
            screen_width(),
            screen_height(),
            layout.s,
            self.items.len(),
            self.time,
            self.scroll,
        )
    }

    /// A new pointer down starts a new gesture; an old release cannot choose a row.
    pub(crate) fn cancel_pointer_input(&mut self) {
        self.gesture = MenuGesture::Cancelled;
    }

    fn reveal_focus(&mut self, layout: &MenuLayout) {
        let row = layout.rows[self.focus];
        let delta = if row.y < layout.viewport.y {
            row.y - layout.viewport.y
        } else if row.bottom() > layout.viewport.bottom() {
            row.bottom() - layout.viewport.bottom()
        } else {
            0.0
        };
        self.scroll = (layout.scroll + delta).clamp(0.0, layout.max_scroll);
    }

    /// The row chosen on pointer down owns a slider drag through its release.
    /// A swipe elsewhere scrolls the list, and never activates its ending row.
    #[must_use]
    fn pointer_action(
        &mut self,
        event: &PointerEvent,
        layout: &MenuLayout,
    ) -> Option<MenuPointerAction> {
        match *event {
            PointerEvent::Down(point) => {
                self.gesture = MenuGesture::Idle;
                if let Some(index) = layout.row_at(point) {
                    let track = slider_rect(self.items[index], layout.rows[index], layout.s);
                    if self.items[index].is_slider() && slider_hit_rect(track).contains(point) {
                        self.focus = index;
                        self.gesture = MenuGesture::Slider { index, track };
                        return Some(MenuPointerAction::SetSlider {
                            index,
                            value: slider_position(track, point.x),
                        });
                    }
                }
                if layout.viewport.contains(point) {
                    self.gesture = MenuGesture::Scroll {
                        start: point,
                        initial_scroll: layout.scroll,
                        moved: false,
                    };
                }
            }
            PointerEvent::Moved(point) => match &mut self.gesture {
                MenuGesture::Slider { index, track } => {
                    return Some(MenuPointerAction::SetSlider {
                        index: *index,
                        value: slider_position(*track, point.x),
                    });
                }
                MenuGesture::Scroll {
                    start,
                    initial_scroll,
                    moved,
                } => {
                    *moved |= start.distance(point) > 6.0;
                    if *moved {
                        self.scroll =
                            (*initial_scroll + start.y - point.y).clamp(0.0, layout.max_scroll);
                    }
                }
                MenuGesture::Idle | MenuGesture::Cancelled => {}
            },
            PointerEvent::Up { end, .. } => {
                let gesture = std::mem::replace(&mut self.gesture, MenuGesture::Idle);
                match gesture {
                    MenuGesture::Slider { index, track } => {
                        return Some(MenuPointerAction::SetSlider {
                            index,
                            value: slider_position(track, end.x),
                        });
                    }
                    MenuGesture::Scroll { moved, start, .. }
                        if moved || start.distance(end) > 6.0 =>
                    {
                        return None;
                    }
                    MenuGesture::Cancelled => return None,
                    _ => {}
                }
                // Keep ordinary click behavior, but never turn a long swipe
                // into a click just because it returned near its starting point.
                if event.click().is_some() {
                    if !layout.card.contains(end) {
                        return Some(MenuPointerAction::Close);
                    }
                    if let Some(index) = layout.row_at(end) {
                        // A slider label can be used as a safe scrolling handle;
                        // only its bar changes the value through pointer input.
                        if !self.items[index].is_slider() {
                            self.focus = index;
                            return Some(MenuPointerAction::Choose(index));
                        }
                    }
                }
            }
        }
        None
    }

    fn set_slider(&self, item: MenuItem, value: f32, ctx: &mut Ctx) {
        let value = (value * 10.0).round().clamp(0.0, 10.0) / 10.0;
        if slider_value(item, ctx.settings) == Some(value) {
            return;
        }
        match item {
            MenuItem::Music => ctx.settings.music_volume = value,
            MenuItem::Sound => ctx.settings.sfx_volume = value,
            MenuItem::Shake => ctx.settings.screen_shake = value,
            MenuItem::TouchRepeat => ctx.settings.set_touch_repeat_slider_value(value),
            _ => return,
        }
        ctx.audio.music_volume = ctx.settings.music_volume;
        ctx.audio.sfx_volume = ctx.settings.sfx_volume;
        // Dragging only saves/plays feedback when crossing into a new step,
        // rather than on every frame spent holding the bar.
        ctx.settings.save();
        ctx.audio.play(Sfx::UiMove);
    }

    fn set_dpad_right(&self, right: bool, ctx: &mut Ctx) {
        if ctx.settings.touch_dpad_right != right {
            ctx.settings.touch_dpad_right = right;
            ctx.settings.save();
            ctx.audio.play(Sfx::UiMove);
        }
    }

    fn set_controls_visible(&self, shown: bool, ctx: &mut Ctx) {
        if ctx.settings.touch_controls_visible != Some(shown) {
            ctx.settings.touch_controls_visible = Some(shown);
            ctx.settings.save();
            ctx.audio.play(Sfx::UiMove);
        }
    }

    fn adjust(&self, item: MenuItem, delta: f32, ctx: &mut Ctx) {
        if item == MenuItem::TouchVisibility {
            self.set_controls_visible(delta > 0.0, ctx);
        } else if item == MenuItem::TouchLayout {
            self.set_dpad_right(delta > 0.0, ctx);
        } else if let Some(current) = slider_value(item, ctx.settings) {
            self.set_slider(item, current + delta, ctx);
        }
    }

    #[must_use]
    fn choose(&self, item: MenuItem, ctx: &mut Ctx) -> Option<MenuChoice> {
        let choice = match item {
            MenuItem::Resume => MenuChoice::Resume,
            MenuItem::Restart => MenuChoice::Restart,
            MenuItem::WorldMap => MenuChoice::WorldMap,
            MenuItem::Export => MenuChoice::Export,
            MenuItem::Import => MenuChoice::Import,
            MenuItem::Title => MenuChoice::Title,
            MenuItem::Quit => MenuChoice::Quit,
            MenuItem::TouchVisibility => {
                self.set_controls_visible(!controls_shown(ctx.settings), ctx);
                return None;
            }
            MenuItem::TouchLayout => {
                self.set_dpad_right(!ctx.settings.touch_dpad_right, ctx);
                return None;
            }
            // Unlike volume, a repeat delay has no on/off state. Adjust it
            // with left/right or the slider; confirmation leaves it alone.
            MenuItem::TouchRepeat => return None,
            MenuItem::Music | MenuItem::Sound | MenuItem::Shake => {
                let on = slider_value(item, ctx.settings).expect("a slider") > 0.0;
                self.set_slider(item, if on { 0.0 } else { 1.0 }, ctx);
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

    #[must_use]
    pub(crate) fn update(
        &mut self,
        ctx: &mut Ctx,
        input: &FrameInput,
        layout: &ScreenLayout,
    ) -> Option<MenuChoice> {
        self.time += input.dt;
        if input.has(MetaInput::Exit) || input.has(MetaInput::Undo) {
            self.cancel_pointer_input();
            ctx.audio.play(Sfx::UiBack);
            return Some(MenuChoice::Resume);
        }
        if input.nav.is_some() || input.confirmed() {
            self.cancel_pointer_input();
        }
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
        if input.nav.is_some() {
            self.reveal_focus(&self.layout(layout));
        }
        if input.confirmed() {
            self.reveal_focus(&self.layout(layout));
            return self.choose(self.items[self.focus], ctx);
        }
        let geometry = self.layout(layout);
        self.scroll = geometry.scroll;
        if input.wheel != 0.0 {
            self.cancel_pointer_input();
            self.scroll =
                (self.scroll - input.wheel * 40.0 * geometry.s).clamp(0.0, geometry.max_scroll);
        }
        for event in &input.pointer {
            match self.pointer_action(event, &self.layout(layout)) {
                Some(MenuPointerAction::SetSlider { index, value }) => {
                    self.set_slider(self.items[index], value, ctx)
                }
                Some(MenuPointerAction::Choose(index)) => {
                    return self.choose(self.items[index], ctx);
                }
                Some(MenuPointerAction::Close) => {
                    ctx.audio.play(Sfx::UiBack);
                    return Some(MenuChoice::Resume);
                }
                None => {}
            }
        }
        None
    }

    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let geometry = self.layout(layout);
        let s = geometry.s;
        let appear = (self.time / 0.18).min(1.0);
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::new(0.02, 0.01, 0.04, 0.6 * appear),
        );
        ui::panel(geometry.card, s);
        text::draw_title(
            sprites,
            "Paused",
            vec2(geometry.card.center().x, geometry.card.y + 46.0 * s),
            40.0 * s,
            GOLD,
            crate::render::palette::INK,
        );
        // Scissors use framebuffer pixels; menu geometry uses logical pixels.
        let dpi = screen_dpi_scale();
        let clip = geometry.viewport;
        unsafe {
            get_internal_gl().quad_gl.scissor(Some((
                (clip.x * dpi).floor() as i32,
                (clip.y * dpi).floor() as i32,
                (clip.w * dpi).ceil() as i32,
                (clip.h * dpi).ceil() as i32,
            )));
        }
        for (i, (item, row)) in self.items.iter().zip(&geometry.rows).enumerate() {
            let row = *row;
            if row.bottom() < clip.y || row.y > clip.bottom() {
                continue;
            }
            let focused = i == self.focus;
            let (icon, label) = match item {
                MenuItem::Resume => (SpriteId::IconPlay, "Resume"),
                MenuItem::Restart => (SpriteId::IconRestart, "Restart level"),
                MenuItem::WorldMap => (SpriteId::IconMap, "World map"),
                MenuItem::Music => (SpriteId::IconMusic, "Music"),
                MenuItem::Sound => (SpriteId::IconSound, "Sound"),
                MenuItem::Shake => (SpriteId::IconGear, "Screen shake"),
                MenuItem::TouchVisibility => (
                    SpriteId::IconGear,
                    if controls_shown(ctx.settings) {
                        "On-screen controls: shown"
                    } else {
                        "On-screen controls: hidden"
                    },
                ),
                MenuItem::TouchLayout => (
                    SpriteId::IconGear,
                    if ctx.settings.touch_dpad_right {
                        "D-pad: right"
                    } else {
                        "D-pad: left"
                    },
                ),
                MenuItem::TouchRepeat => (SpriteId::IconGear, "D-pad delay"),
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
                    let icon = if value == 0.0 && matches!(item, MenuItem::Music | MenuItem::Sound)
                    {
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
                    let track = slider_rect(*item, row, s);
                    let label_x = row.x + 50.0 * s;
                    let label_size = 21.0 * s;
                    let label_width = text::width(sprites, label, Face::Display, label_size);
                    let label_size = label_size
                        * ((slider_hit_rect(track).x - label_x - 6.0 * s) / label_width).min(1.0);
                    text::draw_aligned(
                        sprites,
                        label,
                        vec2(label_x, row.center().y),
                        Align::Left,
                        Face::Display,
                        label_size,
                        TEXT,
                    );
                    ui::slider(track, value, focused, s);
                    let value_label = if *item == MenuItem::TouchRepeat {
                        format!("{:.2}s", ctx.settings.touch_repeat_delay_seconds())
                    } else {
                        format!("{}", (value * 10.0).round() as i32)
                    };
                    let value_size = if *item == MenuItem::TouchRepeat {
                        let available =
                            row.right() - 14.0 * s - slider_hit_rect(track).right() - 6.0 * s;
                        let width = text::width(sprites, &value_label, Face::Display, 18.0 * s);
                        18.0 * s * (available / width).min(1.0)
                    } else {
                        18.0 * s
                    };
                    text::draw_aligned(
                        sprites,
                        &value_label,
                        vec2(row.right() - 14.0 * s, row.center().y),
                        Align::Right,
                        Face::Display,
                        value_size,
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
        unsafe {
            get_internal_gl().quad_gl.scissor(None);
        }
        if geometry.max_scroll > 0.0 && clip.h > 0.0 {
            let x = clip.right() - 4.0 * s;
            rounded_rect(
                Rect::new(x, clip.y, 4.0 * s, clip.h),
                2.0 * s,
                rgb(0x40364e),
            );
            let content_h = clip.h + geometry.max_scroll;
            let thumb_h = (clip.h * clip.h / content_h).max(24.0 * s).min(clip.h);
            let y = clip.y + (clip.h - thumb_h) * geometry.scroll / geometry.max_scroll;
            rounded_rect(Rect::new(x, y, 4.0 * s, thumb_h), 2.0 * s, GOLD);
        }
    }
}

fn slider_rect(item: MenuItem, row: Rect, s: f32) -> Rect {
    let x = row.x + row.w * 0.52;
    // The delay shows a real time value instead of a single slider step.
    let value_space = if item == MenuItem::TouchRepeat {
        84.0
    } else {
        46.0
    };
    Rect::new(
        x,
        row.y,
        (row.right() - x - value_space * s).max(1.0),
        row.h,
    )
}

fn slider_hit_rect(track: Rect) -> Rect {
    // The painted knob extends beyond each endpoint by this radius.
    let radius = track.h * 0.36;
    Rect::new(track.x - radius, track.y, track.w + 2.0 * radius, track.h)
}

fn slider_position(track: Rect, x: f32) -> f32 {
    // Eleven equal zones keep the existing click-to-set steps, with endpoints
    // just as easy to reach as intermediate values. A drag clamps at the ends.
    (((x - track.x) / track.w * 11.0).floor()).clamp(0.0, 10.0) / 10.0
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
        MenuItem::TouchRepeat => Some(settings.touch_repeat_slider_value()),
        _ => None,
    }
}

fn controls_shown(settings: &Settings) -> bool {
    settings
        .touch_controls_visible
        .unwrap_or_else(crate::touch_ui::controls_visible)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Menu {
        let mut menu = Menu::new(true);
        menu.items = vec![
            MenuItem::Resume,
            MenuItem::Restart,
            MenuItem::WorldMap,
            MenuItem::Music,
            MenuItem::Sound,
            MenuItem::Shake,
            MenuItem::TouchVisibility,
            MenuItem::TouchLayout,
            MenuItem::TouchRepeat,
            MenuItem::Export,
            MenuItem::Import,
        ];
        menu.time = 1.0;
        menu
    }

    fn layout(menu: &Menu) -> MenuLayout {
        MenuLayout::new(320.0, 445.0, 0.8, menu.items.len(), menu.time, menu.scroll)
    }

    #[test]
    fn bounded_menu_keeps_touch_targets_on_small_phone_viewports() {
        for (width, height, scale) in [
            (320.0, 445.0, 0.8),
            (393.0, 545.0, 0.9825),
            (393.0, 495.0, 0.9825),
            (468.0, 322.0, 0.42),
            (296.0, 282.0, 0.42),
        ] {
            for time in [0.0, 0.09, 0.18, 1.0] {
                let layout = MenuLayout::new(width, height, scale, menu().items.len(), time, 0.0);
                assert!(layout.card.x >= 0.0 && layout.card.right() <= width);
                assert!(layout.card.y >= 0.0 && layout.card.bottom() <= height);
                assert!(layout.viewport.y >= layout.card.y);
                assert!(layout.viewport.bottom() <= layout.card.bottom());
                assert!(layout.rows.iter().all(|row| row.h >= 44.0));
                assert!(layout.max_scroll > 0.0);
            }
        }
    }

    #[test]
    fn touch_repeat_setting_is_only_in_the_web_menu() {
        for in_level in [false, true] {
            assert_eq!(
                Menu::new(in_level).items.contains(&MenuItem::TouchRepeat),
                cfg!(target_arch = "wasm32")
            );
        }
    }

    #[test]
    fn touch_repeat_slider_keeps_its_scrolled_row_and_clamps_an_outside_release() {
        let mut menu = menu();
        let index = menu
            .items
            .iter()
            .position(|item| *item == MenuItem::TouchRepeat)
            .unwrap();
        menu.focus = index;
        menu.reveal_focus(&layout(&menu));
        let geometry = layout(&menu);
        let track = slider_rect(MenuItem::TouchRepeat, geometry.rows[index], geometry.s);
        let start = track.center();
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(start), &geometry),
            Some(MenuPointerAction::SetSlider { index, value: 0.5 })
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Moved(vec2(track.right() + 100.0, -100.0)),
                &geometry
            ),
            Some(MenuPointerAction::SetSlider { index, value: 1.0 })
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start,
                    end: vec2(-100.0, -100.0)
                },
                &geometry
            ),
            Some(MenuPointerAction::SetSlider { index, value: 0.0 })
        );
        assert_eq!(menu.focus, index);
        assert_eq!(
            slider_value(MenuItem::TouchRepeat, &Settings::default()),
            Some(0.4)
        );
    }

    #[test]
    fn slider_drag_keeps_its_original_row_and_clamps_an_outside_release() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let track = slider_rect(MenuItem::Music, geometry.rows[3], geometry.s);
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(track.center()), &geometry),
            Some(MenuPointerAction::SetSlider {
                index: 3,
                value: 0.5
            })
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Moved(vec2(track.right() + 100.0, geometry.rows[4].center().y)),
                &geometry
            ),
            Some(MenuPointerAction::SetSlider {
                index: 3,
                value: 1.0
            })
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start: track.center(),
                    end: vec2(-100.0, -100.0)
                },
                &geometry
            ),
            Some(MenuPointerAction::SetSlider {
                index: 3,
                value: 0.0
            })
        );
        assert_eq!(
            menu.pointer_action(&PointerEvent::Moved(track.center()), &geometry),
            None
        );
        assert_eq!(menu.focus, 3);
    }

    #[test]
    fn tapping_a_slider_keeps_the_clicked_step_on_release() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let track = slider_rect(MenuItem::Music, geometry.rows[3], geometry.s);
        let point = vec2(track.x + track.w * 0.25, track.center().y);
        let expected = Some(MenuPointerAction::SetSlider {
            index: 3,
            value: 0.2,
        });
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(point), &geometry),
            expected
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start: point,
                    end: point
                },
                &geometry
            ),
            expected
        );
    }

    #[test]
    fn endpoint_knobs_can_be_grabbed_outside_the_bar() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let track = slider_rect(MenuItem::Music, geometry.rows[3], geometry.s);
        for (x, value) in [
            (track.x - track.h * 0.3, 0.0),
            (track.right() + track.h * 0.3, 1.0),
        ] {
            let point = vec2(x, track.center().y);
            assert_eq!(
                menu.pointer_action(&PointerEvent::Down(point), &geometry),
                Some(MenuPointerAction::SetSlider { index: 3, value })
            );
        }
    }

    #[test]
    fn interrupting_a_drag_suppresses_its_release_but_accepts_a_new_gesture() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let track = slider_rect(MenuItem::Music, geometry.rows[3], geometry.s);
        assert!(
            menu.pointer_action(&PointerEvent::Down(track.center()), &geometry)
                .is_some()
        );
        menu.cancel_pointer_input();
        assert_eq!(
            menu.pointer_action(&PointerEvent::Moved(vec2(-100.0, -100.0)), &geometry),
            None
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start: track.center(),
                    end: geometry.rows[0].center()
                },
                &geometry
            ),
            None
        );
        let resume = geometry.rows[0].center();
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(resume), &geometry),
            None
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start: resume,
                    end: resume
                },
                &geometry
            ),
            Some(MenuPointerAction::Choose(0))
        );
    }

    #[test]
    fn swiping_the_list_reveals_hidden_settings_without_activating_a_row() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let start = vec2(
            geometry.viewport.x + 10.0,
            geometry.viewport.bottom() - 10.0,
        );
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(start), &geometry),
            None
        );
        let end = start - vec2(0.0, 240.0);
        assert_eq!(
            menu.pointer_action(&PointerEvent::Moved(end), &geometry),
            None
        );
        assert!(menu.scroll > 0.0);
        let scrolled = layout(&menu);
        assert_eq!(
            menu.pointer_action(&PointerEvent::Up { start, end }, &scrolled),
            None
        );
        for index in [6, 7] {
            let setting = scrolled.rows[index].center();
            assert_eq!(scrolled.row_at(setting), Some(index));
            assert_eq!(
                menu.pointer_action(&PointerEvent::Down(setting), &scrolled),
                None
            );
            assert_eq!(
                menu.pointer_action(
                    &PointerEvent::Up {
                        start: setting,
                        end: setting
                    },
                    &scrolled
                ),
                Some(MenuPointerAction::Choose(index))
            );
        }
    }

    #[test]
    fn a_fast_swipe_without_an_intermediate_move_does_not_click() {
        let mut menu = menu();
        let geometry = layout(&menu);
        let start = geometry.rows[0].center();
        assert_eq!(
            menu.pointer_action(&PointerEvent::Down(start), &geometry),
            None
        );
        assert_eq!(
            menu.pointer_action(
                &PointerEvent::Up {
                    start,
                    end: start + vec2(0.0, 15.0)
                },
                &geometry
            ),
            None
        );
    }

    #[test]
    fn focus_reveal_restores_a_row_scrolled_offscreen() {
        let mut menu = menu();
        let last = menu.items.len() - 1;
        menu.focus = last;
        menu.reveal_focus(&layout(&menu));
        let bottom = layout(&menu);
        assert!(bottom.rows[last].bottom() <= bottom.viewport.bottom() + 0.001);
        menu.focus = 0;
        menu.reveal_focus(&bottom);
        let top = layout(&menu);
        assert_eq!(top.scroll, 0.0);
        assert!(top.rows[0].y >= top.viewport.y);
        // The same reveal operation is used for left/right adjustment, even
        // when the focus itself did not change after a touch/wheel scroll.
        menu.scroll = bottom.max_scroll;
        menu.reveal_focus(&layout(&menu));
        assert_eq!(menu.scroll, 0.0);
    }
}
