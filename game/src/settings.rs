use serde::{Deserialize, Serialize};

use crate::storage;

const SETTINGS: &str = "settings";

/// Player preferences, persisted across sessions.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) music_volume: f32,
    pub(crate) sfx_volume: f32,
    /// The level the hero last stood at on the world map.
    pub(crate) map_level: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music_volume: 0.5,
            sfx_volume: 0.9,
            map_level: None,
        }
    }
}

impl Settings {
    pub(crate) fn load() -> Self {
        storage::load(SETTINGS).unwrap_or_default()
    }

    pub(crate) fn save(&self) {
        storage::save(SETTINGS, self);
    }
}
