//! The game: scenes (title, world map, level), overlays, transitions,
//! music, and input dispatch.

use macroquad::prelude::*;
use quad_gamepad::GamepadContext;

use crate::audio::{Audio, AudioLoader, Sfx, Track};
use crate::input::{InputHints, InputState, PointerEvent, PointerInput};
use crate::levels::{self, Level};
use crate::progress::Progress;
use crate::render::palette::{GOLD, INK, TEXT_DIM, rgb};
use crate::render::shapes::{iris, rounded_rect};
use crate::render::text;
use crate::render::ui::ScreenLayout;
use crate::scenes::level::LevelScene;
use crate::scenes::map::MapScene;
use crate::scenes::menu::{CodeDialog, Menu, MenuChoice};
use crate::scenes::title::TitleScene;
use crate::scenes::{Ctx, FrameInput, Request};
use crate::screen_wake;
use crate::settings::Settings;
use crate::sprites::Sprites;
use crate::storage;
use crate::world_map::Theme;

enum Scene {
    Title(TitleScene),
    Map(Box<MapScene>),
    Level(Box<LevelScene>),
}

impl Scene {
    fn track(&self) -> Track {
        match self {
            Scene::Title(_) | Scene::Map(_) => Track::TitleMap,
            Scene::Level(level) => match level.theme() {
                Theme::Cellar | Theme::Archive => Track::Dungeon,
                Theme::Powder | Theme::Hall | Theme::Towers => Track::Keep,
                Theme::Lab => Track::Lab,
            },
        }
    }

    /// Where an iris transition should close/open around.
    fn focus(&self, layout: &ScreenLayout) -> Vec2 {
        match self {
            Scene::Title(_) => vec2(screen_width(), screen_height()) / 2.0,
            Scene::Map(map) => map.focus(layout),
            Scene::Level(level) => level.focus(layout),
        }
    }
}

enum Overlay {
    Menu(Menu),
    Code(CodeDialog),
}

/// An iris wipe: closes on the old scene, swaps, opens on the new one.
struct Transition {
    time: f32,
    next: Option<Scene>,
}

const IRIS_CLOSE: f32 = 0.34;
const IRIS_OPEN: f32 = 0.42;

pub struct App {
    sprites: Sprites,
    audio: Audio,
    settings: Settings,
    progress: Progress,
    input: InputState,
    gamepad: GamepadContext,
    scene: Scene,
    overlay: Option<Overlay>,
    transition: Option<Transition>,
    /// A clipboard import is in progress (async on the web).
    import_pending: bool,
    quit: bool,
}

/// Load sounds while showing a progress bar. Returns once the game can start.
async fn load_audio(sprites: &Sprites) -> AudioLoader {
    let loader = AudioLoader::start();
    while !loader.ready() {
        clear_background(rgb(0x07060a));
        let s = ScreenLayout::current().s;
        let center = vec2(screen_width(), screen_height()) / 2.0;
        text::draw_title(
            sprites,
            "INFESTATION",
            center - vec2(0.0, 40.0 * s),
            54.0 * s,
            GOLD,
            INK,
        );
        let bar = Rect::new(
            center.x - 140.0 * s,
            center.y + 20.0 * s,
            280.0 * s,
            10.0 * s,
        );
        rounded_rect(bar, 5.0 * s, rgb(0x231e2e));
        rounded_rect(
            Rect::new(bar.x, bar.y, (bar.w * loader.progress()).max(bar.h), bar.h),
            5.0 * s,
            TEXT_DIM,
        );
        next_frame().await;
    }
    loader
}

#[cfg(test)]
pub(crate) async fn load_audio_for_tests(sprites: &Sprites) -> Audio {
    load_audio(sprites).await.finish(0.0, 0.0)
}

impl App {
    /// Load the game's assets, then start at the title screen (or straight
    /// in a level, if one is named).
    pub async fn load(level_name: Option<&str>) -> Self {
        let sprites = Sprites::load().await;
        let settings = Settings::load();
        let audio = load_audio(&sprites)
            .await
            .finish(settings.music_volume, settings.sfx_volume);
        Self::new(sprites, audio, settings, Progress::load(), level_name)
    }

    pub(crate) fn new(
        sprites: Sprites,
        audio: Audio,
        settings: Settings,
        progress: Progress,
        level_name: Option<&str>,
    ) -> Self {
        screen_wake::request();
        let scene = match level_name {
            Some(name) => Scene::Level(Box::new(LevelScene::new(
                levels::get_level(name).unwrap_or_else(|| panic!("Level not found: {name}")),
            ))),
            None => Scene::Title(TitleScene::new()),
        };
        Self {
            sprites,
            audio,
            settings,
            progress,
            input: InputState::new(),
            gamepad: GamepadContext::new(),
            scene,
            overlay: None,
            transition: None,
            import_pending: false,
            quit: false,
        }
    }

    fn go_to(&mut self, next: Scene) {
        self.input.reset();
        self.transition = Some(Transition {
            time: 0.0,
            next: Some(next),
        });
    }

    fn map_scene(&self, from: Option<&'static Level>, before: Option<&Progress>) -> Scene {
        Scene::Map(Box::new(MapScene::new(
            &self.progress,
            self.settings.map_level.as_deref(),
            from,
            before,
        )))
    }

    fn poll_input(&mut self, dt: f32) -> FrameInput {
        let players = match &self.scene {
            Scene::Level(level) => level.player_count(),
            _ => 1,
        };
        self.input.track_device(&self.gamepad);
        let meta = self.input.poll_meta_inputs(&self.gamepad, dt);
        let player_actions = self.input.poll_player_actions(&self.gamepad, dt, players);
        let nav = self.input.poll_nav(&self.gamepad, dt);
        let PointerInput {
            events: pointer,
            wheel,
        } = self.input.poll_pointer();
        let any = !meta.is_empty()
            || player_actions.values().any(Option::is_some)
            || nav.is_some()
            || get_last_key_pressed().is_some()
            || pointer.iter().any(|e| matches!(e, PointerEvent::Up { .. }));
        FrameInput {
            dt,
            meta,
            players: player_actions,
            nav,
            pointer,
            wheel,
            any,
        }
    }

    /// Run one frame. Returns false when the game should exit.
    pub fn tick(&mut self) -> bool {
        self.gamepad.poll();
        let dt = get_frame_time().min(1.0 / 20.0);
        let input = self.poll_input(dt);
        let hints = self.input.hints(&self.gamepad);
        let gamepads = self.gamepad.connected_count();
        self.step(&input, hints, gamepads);
        self.draw(&ScreenLayout::current(), hints, gamepads);
        self.gamepad.end_frame();
        !self.quit
    }

    /// Advance everything by one frame of input.
    pub(crate) fn step(&mut self, input: &FrameInput, hints: InputHints, gamepads: usize) {
        let dt = input.dt;
        let layout = ScreenLayout::current();
        let mut ctx = Ctx {
            sprites: &self.sprites,
            audio: &mut self.audio,
            progress: &mut self.progress,
            settings: &mut self.settings,
            hints,
            gamepads,
        };

        let mut request = None;
        let mut choice = None;
        if let Some(transition) = &mut self.transition {
            transition.time += dt;
            if transition.time >= IRIS_CLOSE
                && let Some(next) = transition.next.take()
            {
                self.scene = next;
            }
            if transition.time >= IRIS_CLOSE + IRIS_OPEN {
                self.transition = None;
            }
        } else if let Some(overlay) = &mut self.overlay {
            match overlay {
                Overlay::Menu(menu) => choice = menu.update(&mut ctx, input, &layout),
                Overlay::Code(dialog) => {
                    if dialog.update(&mut ctx, input, &layout) {
                        self.overlay = None;
                    }
                }
            }
        } else {
            match &mut self.scene {
                Scene::Title(title) => {
                    if title.update(input) {
                        ctx.audio.play(Sfx::UiConfirm);
                        request = Some(Request::ToMap {
                            from_level: None,
                            before: None,
                        });
                    }
                }
                Scene::Map(map) => request = map.update(&mut ctx, input, &layout),
                Scene::Level(level) => {
                    request = level.update(&mut ctx, input, &layout);
                    if let Some(code) = level.solution.take() {
                        self.overlay = Some(Overlay::Code(CodeDialog::new("Solution code", code)));
                    }
                }
            }
        }

        match request {
            Some(Request::PlayLevel(level)) => {
                self.go_to(Scene::Level(Box::new(LevelScene::new(level))));
            }
            Some(Request::ToMap { from_level, before }) => {
                let map = self.map_scene(from_level, before.as_ref());
                self.go_to(map);
            }
            Some(Request::Pause) => {
                self.audio.play(Sfx::Pause);
                self.overlay = Some(Overlay::Menu(Menu::new(matches!(
                    self.scene,
                    Scene::Level(_)
                ))));
            }
            None => {}
        }
        if let Some(choice) = choice {
            self.on_menu_choice(choice, hints, gamepads);
        }

        if self.import_pending
            && let Some(imported) = storage::progress::poll_import()
        {
            self.progress.import(imported);
            self.import_pending = false;
            self.audio.play(Sfx::Unlock);
            if let Scene::Map(_) = self.scene {
                self.scene = self.map_scene(None, None);
            }
        }

        let track = self
            .transition
            .as_ref()
            .and_then(|t| t.next.as_ref())
            .unwrap_or(&self.scene)
            .track();
        self.audio.set_track(Some(track));
        self.audio.update(dt);
    }

    fn on_menu_choice(&mut self, choice: MenuChoice, hints: InputHints, gamepads: usize) {
        self.overlay = None;
        match choice {
            MenuChoice::Resume => {}
            MenuChoice::Restart => {
                if let Scene::Level(level) = &mut self.scene {
                    let mut ctx = Ctx {
                        sprites: &self.sprites,
                        audio: &mut self.audio,
                        progress: &mut self.progress,
                        settings: &mut self.settings,
                        hints,
                        gamepads,
                    };
                    level.restart(&mut ctx);
                }
            }
            MenuChoice::WorldMap => {
                let from = match &self.scene {
                    Scene::Level(level) => Some(level.level),
                    _ => None,
                };
                let map = self.map_scene(from, None);
                self.go_to(map);
            }
            MenuChoice::Export => {
                self.overlay = Some(Overlay::Code(CodeDialog::new(
                    "Your progress",
                    self.progress.export(),
                )));
            }
            MenuChoice::Import => {
                storage::progress::start_import();
                self.import_pending = true;
            }
            MenuChoice::Title => self.go_to(Scene::Title(TitleScene::new())),
            MenuChoice::Quit => self.quit = true,
        }
    }

    pub(crate) fn draw(&mut self, layout: &ScreenLayout, hints: InputHints, gamepads: usize) {
        let ctx = Ctx {
            sprites: &self.sprites,
            audio: &mut self.audio,
            progress: &mut self.progress,
            settings: &mut self.settings,
            hints,
            gamepads,
        };
        match &self.scene {
            Scene::Title(title) => title.draw(&ctx, layout),
            Scene::Map(map) => map.draw(&ctx, layout),
            Scene::Level(level) => level.draw(&ctx, layout),
        }
        match &self.overlay {
            Some(Overlay::Menu(menu)) => menu.draw(&ctx, layout),
            Some(Overlay::Code(dialog)) => dialog.draw(&ctx, layout),
            None => {}
        }
        if let Some(t) = &self.transition {
            let max = vec2(screen_width(), screen_height()).length();
            let radius = if t.time < IRIS_CLOSE {
                let k = t.time / IRIS_CLOSE;
                max * (1.0 - k * k)
            } else {
                let k = ((t.time - IRIS_CLOSE) / IRIS_OPEN).min(1.0);
                max * (1.0 - (1.0 - k).powi(3))
            };
            iris(self.scene.focus(layout), radius, rgb(0x07060a));
        }
        self.sprites.finish_frame();
    }
}
