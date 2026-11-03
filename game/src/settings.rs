use serde::{Deserialize, Serialize};

use crate::storage;

const SETTINGS: &str = "settings";
const TOUCH_REPEAT_MIN_MS: u16 = 100;
const TOUCH_REPEAT_MAX_MS: u16 = 600;
const TOUCH_REPEAT_STEP_MS: u16 = 50;

/// Player preferences, persisted across sessions.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) music_volume: f32,
    pub(crate) sfx_volume: f32,
    /// How much the screen shakes, from 0 (not at all) to 1 (fully).
    pub(crate) screen_shake: f32,
    /// Put the touch D-pad on the right and the action buttons on the left.
    pub(crate) touch_dpad_right: bool,
    /// Explicit on-screen control preference; None keeps the device default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) touch_controls_visible: Option<bool>,
    /// Delay before and between held touch D-pad moves. Fresh taps stay immediate.
    pub(crate) touch_repeat_delay_ms: u16,
    /// The level the hero last stood at on the world map.
    pub(crate) map_level: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music_volume: 0.5,
            sfx_volume: 0.9,
            screen_shake: 1.0,
            touch_dpad_right: false,
            touch_controls_visible: None,
            touch_repeat_delay_ms: 300,
            map_level: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_keep_their_values_and_default_to_the_left_dpad() {
        let settings: Settings = serde_json::from_str(
            r#"{"music_volume":0.2,"sfx_volume":0.7,"screen_shake":0.4,"map_level":"planks"}"#,
        )
        .unwrap();
        assert!(!settings.touch_dpad_right);
        assert_eq!(settings.touch_controls_visible, None);
        assert_eq!(settings.touch_repeat_delay_ms, 300);
        assert_eq!(settings.music_volume, 0.2);
        assert_eq!(settings.sfx_volume, 0.7);
        assert_eq!(settings.screen_shake, 0.4);
        assert_eq!(settings.map_level.as_deref(), Some("planks"));
    }

    #[test]
    fn dpad_side_round_trips_with_the_other_settings() {
        let settings = Settings {
            touch_dpad_right: true,
            music_volume: 0.3,
            ..Settings::default()
        };
        let loaded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(loaded.touch_dpad_right);
        assert_eq!(loaded.music_volume, 0.3);
        assert_eq!(loaded.sfx_volume, settings.sfx_volume);
        assert_eq!(loaded.screen_shake, settings.screen_shake);
    }

    #[test]
    fn control_visibility_round_trips_without_losing_device_defaults() {
        for preference in [None, Some(false), Some(true)] {
            let settings = Settings {
                touch_controls_visible: preference,
                touch_dpad_right: true,
                music_volume: 0.3,
                ..Settings::default()
            };
            let saved = serde_json::to_value(&settings).unwrap();
            if preference.is_none() {
                assert!(saved.get("touch_controls_visible").is_none());
            }
            let loaded: Settings = serde_json::from_value(saved).unwrap();
            assert_eq!(loaded.touch_controls_visible, preference);
            assert!(loaded.touch_dpad_right);
            assert_eq!(loaded.music_volume, 0.3);
        }
    }

    #[test]
    fn touch_repeat_delay_round_trips_with_existing_preferences() {
        let settings = Settings {
            touch_repeat_delay_ms: 450,
            touch_dpad_right: true,
            music_volume: 0.2,
            map_level: Some("planks".into()),
            ..Settings::default()
        };
        let loaded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(loaded.touch_repeat_delay_ms, 450);
        assert_eq!(loaded.touch_repeat_delay_seconds(), 0.45);
        assert!(loaded.touch_dpad_right);
        assert_eq!(loaded.music_volume, 0.2);
        assert_eq!(loaded.map_level.as_deref(), Some("planks"));
    }

    #[test]
    fn out_of_range_touch_delays_are_bounded_without_losing_other_preferences() {
        for (saved, seconds, slider) in [(0, 0.1, 0.0), (1000, 0.6, 1.0)] {
            let settings: Settings = serde_json::from_value(serde_json::json!({
                "touch_repeat_delay_ms": saved,
                "music_volume": 0.2,
                "map_level": "planks"
            }))
            .unwrap();
            assert_eq!(settings.touch_repeat_delay_seconds(), seconds);
            assert_eq!(settings.touch_repeat_slider_value(), slider);
            assert_eq!(settings.music_volume, 0.2);
            assert_eq!(settings.map_level.as_deref(), Some("planks"));
        }
    }

    #[test]
    fn touch_delay_slider_uses_fifty_millisecond_steps_and_clamps_at_both_ends() {
        let mut settings = Settings::default();
        assert_eq!(settings.touch_repeat_slider_value(), 0.4);
        for (value, milliseconds) in [
            (-0.2, 100),
            (0.0, 100),
            (0.1, 150),
            (0.46, 350),
            (0.9, 550),
            (1.0, 600),
            (1.2, 600),
        ] {
            settings.set_touch_repeat_slider_value(value);
            assert_eq!(settings.touch_repeat_delay_ms, milliseconds);
        }
    }
}

impl Settings {
    pub(crate) fn load() -> Self {
        let mut settings: Self = storage::load(SETTINGS).unwrap_or_default();
        settings.touch_repeat_delay_ms = settings.bounded_touch_repeat_delay_ms();
        settings
    }

    pub(crate) fn save(&self) {
        storage::save(SETTINGS, self);
    }

    fn bounded_touch_repeat_delay_ms(&self) -> u16 {
        self.touch_repeat_delay_ms
            .clamp(TOUCH_REPEAT_MIN_MS, TOUCH_REPEAT_MAX_MS)
    }

    pub(crate) fn touch_repeat_delay_seconds(&self) -> f32 {
        f32::from(self.bounded_touch_repeat_delay_ms()) / 1000.0
    }

    pub(crate) fn touch_repeat_slider_value(&self) -> f32 {
        f32::from(self.bounded_touch_repeat_delay_ms() - TOUCH_REPEAT_MIN_MS)
            / f32::from(TOUCH_REPEAT_MAX_MS - TOUCH_REPEAT_MIN_MS)
    }

    pub(crate) fn set_touch_repeat_slider_value(&mut self, value: f32) {
        let step = (value * 10.0).round().clamp(0.0, 10.0) as u16;
        self.touch_repeat_delay_ms = TOUCH_REPEAT_MIN_MS + step * TOUCH_REPEAT_STEP_MS;
    }
}
