//! Sound effects and music.
//!
//! Effects are embedded and decoded at startup; music streams in from
//! `assets/music/` in the background and cross-fades between scenes.

use enum_map::{Enum, EnumMap};
use macroquad::audio::{
    PlaySoundParams, Sound, load_sound, load_sound_from_bytes, play_sound, set_sound_volume,
    stop_sound,
};
use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
use macroquad::rand::gen_range;

/// A sound effect; some have several variants picked at random.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub(crate) enum Sfx {
    Step,
    Bump,
    Scurry,
    Squeak,
    Slash,
    Hit,
    HeroDeath,
    Servo,
    CyborgDeath,
    Chomp,
    Explosion,
    Zap,
    WallRise,
    PlankBreak,
    WebTear,
    Swallow,
    Trigger,
    Undo,
    Restart,
    UiMove,
    UiConfirm,
    UiBack,
    UiLocked,
    MapStep,
    TextBlip,
    LevelStart,
    Pause,
    Win,
    Lose,
    Unlock,
    Reveal,
}

/// The web uses MP3 (every browser decodes it; Safari may not decode Ogg
/// Vorbis), native uses Ogg Vorbis (the native decoder can't read MP3).
#[cfg(target_arch = "wasm32")]
macro_rules! audio_ext {
    () => {
        "mp3"
    };
}
#[cfg(not(target_arch = "wasm32"))]
macro_rules! audio_ext {
    () => {
        "ogg"
    };
}

macro_rules! sfx {
    ($($name:literal),+) => {
        &[$(include_bytes!(concat!("../../assets/sfx/", $name, ".", audio_ext!()))),+]
    };
}

fn sfx_files(sfx: Sfx) -> &'static [&'static [u8]] {
    match sfx {
        Sfx::Step => sfx!("step_1", "step_2", "step_3"),
        Sfx::Bump => sfx!("bump"),
        Sfx::Scurry => sfx!("scurry_1", "scurry_2", "scurry_3"),
        Sfx::Squeak => sfx!("squeak_1", "squeak_2", "squeak_3"),
        Sfx::Slash => sfx!("slash"),
        Sfx::Hit => sfx!("hit"),
        Sfx::HeroDeath => sfx!("hero_death"),
        Sfx::Servo => sfx!("servo_1", "servo_2"),
        Sfx::CyborgDeath => sfx!("cyborg_death"),
        Sfx::Chomp => sfx!("chomp"),
        Sfx::Explosion => sfx!("explosion"),
        Sfx::Zap => sfx!("zap"),
        Sfx::WallRise => sfx!("wall_rise"),
        Sfx::PlankBreak => sfx!("plank_break"),
        Sfx::WebTear => sfx!("web_tear"),
        Sfx::Swallow => sfx!("swallow"),
        Sfx::Trigger => sfx!("trigger"),
        Sfx::Undo => sfx!("undo"),
        Sfx::Restart => sfx!("restart"),
        Sfx::UiMove => sfx!("ui_move"),
        Sfx::UiConfirm => sfx!("ui_confirm"),
        Sfx::UiBack => sfx!("ui_back"),
        Sfx::UiLocked => sfx!("ui_locked"),
        Sfx::MapStep => sfx!("map_step"),
        Sfx::TextBlip => sfx!("text_blip_1", "text_blip_2", "text_blip_3"),
        Sfx::LevelStart => sfx!("level_start"),
        Sfx::Pause => sfx!("pause"),
        Sfx::Win => sfx!("win"),
        Sfx::Lose => sfx!("lose"),
        Sfx::Unlock => sfx!("unlock"),
        Sfx::Reveal => sfx!("reveal"),
    }
}

/// A looping music track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enum)]
pub(crate) enum Track {
    TitleMap,
    Dungeon,
    Keep,
    Lab,
}

impl Track {
    fn path(self) -> &'static str {
        match self {
            Track::TitleMap => concat!("assets/music/title_map.", audio_ext!()),
            Track::Dungeon => concat!("assets/music/dungeon.", audio_ext!()),
            Track::Keep => concat!("assets/music/keep.", audio_ext!()),
            Track::Lab => concat!("assets/music/lab.", audio_ext!()),
        }
    }
}

/// Seconds for music to fade in or out.
const FADE: f32 = 1.2;

struct Music {
    sound: Option<Sound>,
    loading: Option<Coroutine<Sound>>,
    /// Current fade level in [0, 1].
    level: f32,
    playing: bool,
}

pub(crate) struct Audio {
    sfx: EnumMap<Sfx, Vec<Sound>>,
    music: EnumMap<Track, Music>,
    current: Option<Track>,
    /// User volume settings in [0, 1].
    pub(crate) music_volume: f32,
    pub(crate) sfx_volume: f32,
    /// Effects already started this frame, so simultaneous events (e.g.
    /// twenty rats moving) play one sound.
    played: EnumMap<Sfx, bool>,
}

/// Loads every effect (concurrently on the web) and starts the music
/// downloads.
pub(crate) struct AudioLoader {
    sfx: Vec<(Sfx, Coroutine<Sound>)>,
    music: EnumMap<Track, Option<Coroutine<Sound>>>,
}

fn load_bytes(bytes: &'static [u8]) -> Coroutine<Sound> {
    start_coroutine(async move {
        load_sound_from_bytes(bytes)
            .await
            .expect("bundled sound should decode")
    })
}

impl AudioLoader {
    pub(crate) fn start() -> Self {
        let sfx = (0..Sfx::LENGTH)
            .map(Sfx::from_usize)
            .flat_map(|s| {
                sfx_files(s)
                    .iter()
                    .map(move |&bytes| (s, load_bytes(bytes)))
            })
            .collect();
        let music = EnumMap::from_fn(|track: Track| {
            Some(start_coroutine(async move {
                load_sound(track.path())
                    .await
                    .unwrap_or_else(|e| panic!("failed to load {}: {e:?}", track.path()))
            }))
        });
        Self { sfx, music }
    }

    /// Fraction of the effects decoded.
    pub(crate) fn progress(&self) -> f32 {
        let done = self.sfx.iter().filter(|(_, c)| c.is_done()).count();
        done as f32 / self.sfx.len() as f32
    }

    /// Whether the game can start. Native builds also wait for music, which
    /// decodes on the main thread and would otherwise stall the title screen.
    pub(crate) fn ready(&self) -> bool {
        let sfx_ready = self.sfx.iter().all(|(_, c)| c.is_done());
        let music_ready =
            cfg!(target_arch = "wasm32") || self.music.values().flatten().all(|c| c.is_done());
        sfx_ready && music_ready
    }

    pub(crate) fn finish(self, music_volume: f32, sfx_volume: f32) -> Audio {
        assert!(self.ready());
        let mut sfx: EnumMap<Sfx, Vec<Sound>> = EnumMap::default();
        for (s, coroutine) in self.sfx {
            sfx[s].push(coroutine.retrieve().expect("sound finished loading"));
        }
        let mut loading = self.music;
        Audio {
            sfx,
            music: EnumMap::from_fn(|track| Music {
                sound: None,
                loading: loading[track].take(),
                level: 0.0,
                playing: false,
            }),
            current: None,
            music_volume,
            sfx_volume,
            played: EnumMap::default(),
        }
    }
}

impl Audio {
    /// Play an effect (a random variant), once per frame at most.
    pub(crate) fn play(&mut self, sfx: Sfx) {
        self.play_at(sfx, 1.0);
    }

    /// Play an effect at a relative volume.
    pub(crate) fn play_at(&mut self, sfx: Sfx, volume: f32) {
        if self.played[sfx] {
            return;
        }
        self.played[sfx] = true;
        let variants = &self.sfx[sfx];
        let sound = &variants[gen_range(0, variants.len())];
        play_sound(
            sound,
            PlaySoundParams {
                looped: false,
                volume: volume * self.sfx_volume,
            },
        );
    }

    /// Cross-fade to a track (or to silence).
    pub(crate) fn set_track(&mut self, track: Option<Track>) {
        self.current = track;
    }

    /// Advance fades and pick up finished downloads; call once per frame.
    pub(crate) fn update(&mut self, dt: f32) {
        self.played = EnumMap::default();
        for (track, music) in self.music.iter_mut() {
            if let Some(coroutine) = &music.loading
                && coroutine.is_done()
            {
                music.sound = music.loading.take().and_then(|c| c.retrieve());
            }
            let Some(sound) = &music.sound else {
                continue;
            };
            let target = if self.current == Some(track) {
                1.0
            } else {
                0.0
            };
            let step = dt / FADE;
            music.level = if target > music.level {
                (music.level + step).min(target)
            } else {
                (music.level - step).max(target)
            };
            if music.level > 0.0 && !music.playing {
                play_sound(
                    sound,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.0,
                    },
                );
                music.playing = true;
            } else if music.level == 0.0 && music.playing {
                stop_sound(sound);
                music.playing = false;
            }
            if music.playing {
                set_sound_volume(sound, music.level * music.level * self.music_volume);
            }
        }
    }
}
