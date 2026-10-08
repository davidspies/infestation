//! The game's screens and what they share.

pub(crate) mod level;
pub(crate) mod map;
pub(crate) mod menu;
pub(crate) mod title;

use enum_map::EnumMap;
use macroquad::math::Vec2;

use crate::audio::Audio;
use crate::direction::Dir4;
use crate::grid::Player;
use crate::input::{InputHints, MetaInput, PlayerInput, PointerEvent};
use crate::levels::Level;
use crate::progress::Progress;
use crate::settings::Settings;
use crate::sprites::Sprites;

/// State shared by every scene.
pub(crate) struct Ctx<'a> {
    pub(crate) sprites: &'a Sprites,
    pub(crate) audio: &'a mut Audio,
    pub(crate) progress: &'a mut Progress,
    pub(crate) settings: &'a mut Settings,
    /// Which device's prompts to show.
    pub(crate) hints: InputHints,
    /// Connected controllers (some notes depend on it).
    pub(crate) gamepads: usize,
}

/// One frame of input.
pub(crate) struct FrameInput {
    pub(crate) dt: f32,
    pub(crate) meta: Vec<MetaInput>,
    pub(crate) players: EnumMap<Player, Option<PlayerInput>>,
    /// Menu/map navigation.
    pub(crate) nav: Option<Dir4>,
    pub(crate) pointer: Vec<PointerEvent>,
    /// Any key, button, click or tap (for "press any key").
    pub(crate) any: bool,
}

impl FrameInput {
    /// Taps and clicks this frame (not swipes or drags).
    pub(crate) fn taps(&self) -> impl Iterator<Item = Vec2> + '_ {
        self.pointer.iter().filter_map(|e| match e.gesture()? {
            crate::input::TouchGesture::Tap(pos) => Some(pos),
            crate::input::TouchGesture::Swipe(_) => None,
        })
    }

    pub(crate) fn confirmed(&self) -> bool {
        self.meta.iter().any(|m| matches!(m, MetaInput::Confirm(_)))
    }

    pub(crate) fn has(&self, input: MetaInput) -> bool {
        self.meta.contains(&input)
    }
}

/// What a scene asks the app to do next.
pub(crate) enum Request {
    PlayLevel(&'static Level),
    /// Go to the world map, celebrating newly opened paths if a level was
    /// just completed.
    ToMap {
        from_level: Option<&'static Level>,
        before: Option<Progress>,
    },
    /// Open the pause menu.
    Pause,
}
