use std::collections::HashMap;

use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

use crate::position::Position;

use crate::input::InputHints;

use super::{Cell, Grid};

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum NoteText {
    Simple(String),
    ByPlatform {
        controller: String,
        keyboard: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        touch: Option<String>,
    },
    TwoPlayer {
        keyboard_only: String,
        one_controller: String,
        two_controllers: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        touch: Option<String>,
    },
}

impl NoteText {
    pub(crate) fn resolve(&self, hints: InputHints, gamepad_count: usize) -> &str {
        match self {
            NoteText::Simple(s) => s,
            NoteText::ByPlatform {
                controller,
                keyboard,
                touch,
            } => match hints {
                InputHints::Controller(_) => controller,
                InputHints::Keyboard => keyboard,
                // Legacy device-specific notes lack a touch translation. Give
                // usable touch controls rather than showing physical bindings.
                // Plain notes remain unchanged so their gameplay advice survives.
                InputHints::Touch => touch
                    .as_deref()
                    .unwrap_or("Use the D-pad to move. Tap Wait to pass a turn."),
            },
            NoteText::TwoPlayer {
                keyboard_only,
                one_controller,
                two_controllers,
                touch,
            } => {
                // Touch selects its own wording even with controllers connected.
                if hints == InputHints::Touch {
                    touch.as_deref().unwrap_or(
                        "Tap a hero's portrait, then use the D-pad to move them. To move both heroes together, turn on Move together and choose a direction for each hero.",
                    )
                } else if gamepad_count >= 2 {
                    two_controllers
                } else if gamepad_count == 1 {
                    one_controller
                } else {
                    keyboard_only
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct LevelMetadata {
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<Note>,
}

#[derive(Serialize, Deserialize)]
struct Note {
    x: i32,
    y: i32,
    text: NoteText,
}

impl LevelMetadata {
    pub(crate) fn parse(json_str: &str) -> Self {
        serde_json::from_str(json_str).expect("invalid JSON")
    }

    pub(crate) fn from_grid(name: &str, notes: &HashMap<Position, NoteText>) -> Self {
        let mut notes: Vec<_> = notes
            .iter()
            .map(|(pos, text)| Note {
                x: pos.x,
                y: pos.y,
                text: text.clone(),
            })
            .collect();
        notes.sort_by_key(|n| (n.y, n.x));

        Self {
            name: name.to_string(),
            notes,
        }
    }

    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Failed to serialize JSON")
    }

    pub(crate) fn notes(&self) -> HashMap<Position, NoteText> {
        self.notes
            .iter()
            .map(|n| (Position::new(n.x as usize, n.y as usize), n.text.clone()))
            .collect()
    }
}

impl Grid {
    pub fn from_csv(csv_str: &str) -> Self {
        Self::parse_csv(csv_str, HashMap::new())
    }

    pub(crate) fn from_csv_and_metadata(csv_str: &str, metadata: &LevelMetadata) -> Self {
        Self::parse_csv(csv_str, metadata.notes())
    }

    fn parse_csv(csv_str: &str, notes: HashMap<Position, NoteText>) -> Self {
        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .flexible(false)
            .from_reader(csv_str.as_bytes());

        let cells = reader
            .records()
            .map(|result| {
                let record = result.expect("invalid CSV");
                record
                    .iter()
                    .map(|field| Cell::from_symbol(field.trim()))
                    .collect()
            })
            .collect();

        let mut grid = Grid::new(cells, notes);

        // Point each rat toward a player (P1 if present, otherwise P2)
        let target = grid.find_players().first().map(|p| p.pos);
        let rats: Vec<_> = grid
            .find_entities(|cell| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)))
            .collect();
        for (pos, cell) in rats {
            let target = target.expect("grid has rats but no players");
            let dir = pos.direction_to(target).unwrap();
            *grid.at_mut(pos) = match cell {
                Cell::Rat(_) => Cell::Rat(dir),
                _ => Cell::CyborgRat(dir),
            };
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quad_gamepad::ControllerType;

    fn note(value: serde_json::Value) -> NoteText {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn platform_notes_follow_the_active_input_device() {
        let note = note(serde_json::json!({
            "controller": "Controller movement",
            "keyboard": "Keyboard movement",
            "touch": "Touch movement"
        }));
        assert_eq!(note.resolve(InputHints::Keyboard, 0), "Keyboard movement");
        assert_eq!(
            note.resolve(InputHints::Controller(ControllerType::PlayStation), 1),
            "Controller movement"
        );
        for count in 0..=2 {
            assert_eq!(note.resolve(InputHints::Touch, count), "Touch movement");
        }
    }

    #[test]
    fn two_hero_touch_notes_do_not_depend_on_connected_controllers() {
        let note = note(serde_json::json!({
            "keyboard_only": "Both use keys",
            "one_controller": "One uses a controller",
            "two_controllers": "Both use controllers",
            "touch": "Choose each hero's portrait"
        }));
        for (count, expected) in [
            (0, "Both use keys"),
            (1, "One uses a controller"),
            (2, "Both use controllers"),
        ] {
            assert_eq!(note.resolve(InputHints::Keyboard, count), expected);
            assert_eq!(
                note.resolve(InputHints::Controller(ControllerType::Xbox), count),
                expected
            );
            assert_eq!(
                note.resolve(InputHints::Touch, count),
                "Choose each hero's portrait"
            );
        }
    }

    #[test]
    fn legacy_device_notes_get_touch_help_without_rewriting_plain_notes() {
        let platform = note(serde_json::json!({
            "controller": "Use the joystick",
            "keyboard": "Use WASD"
        }));
        assert_eq!(
            platform.resolve(InputHints::Touch, 0),
            "Use the D-pad to move. Tap Wait to pass a turn."
        );
        let two_hero = note(serde_json::json!({
            "keyboard_only": "Hold Shift",
            "one_controller": "Hold RB or Shift",
            "two_controllers": "Hold RB"
        }));
        for count in 0..=2 {
            assert_eq!(
                two_hero.resolve(InputHints::Touch, count),
                "Tap a hero's portrait, then use the D-pad to move them. To move both heroes together, turn on Move together and choose a direction for each hero."
            );
        }
        let plain = NoteText::Simple("A trigger which is consumed doesn't activate".into());
        assert_eq!(
            plain.resolve(InputHints::Touch, 2),
            "A trigger which is consumed doesn't activate"
        );
    }

    #[test]
    fn editing_round_trips_touch_variants_and_legacy_note_shapes() {
        for original in [
            serde_json::json!({ "controller": "Pad", "keyboard": "Keys" }),
            serde_json::json!({ "controller": "Pad", "keyboard": "Keys", "touch": "Touch" }),
            serde_json::json!({
                "keyboard_only": "Keys", "one_controller": "One pad", "two_controllers": "Two pads"
            }),
            serde_json::json!({
                "keyboard_only": "Keys", "one_controller": "One pad", "two_controllers": "Two pads", "touch": "Portraits"
            }),
            serde_json::json!("Plain gameplay advice"),
        ] {
            let mut notes = HashMap::new();
            notes.insert(Position::new(2, 3), note(original.clone()));
            let saved = LevelMetadata::from_grid("Custom room", &notes).to_json();
            let value: serde_json::Value = serde_json::from_str(&saved).unwrap();
            assert_eq!(value["notes"][0]["text"], original);
            let loaded = LevelMetadata::parse(&saved);
            let loaded_notes = loaded.notes();
            let reloaded = loaded_notes.get(&Position::new(2, 3)).unwrap();
            assert_eq!(
                reloaded.resolve(InputHints::Touch, 2),
                notes[&Position::new(2, 3)].resolve(InputHints::Touch, 2)
            );
        }
    }

    #[test]
    fn every_shipped_device_specific_note_has_touch_wording() {
        let mut device_notes = 0;
        for level in crate::levels::all() {
            for (_, note) in level.grid.notes() {
                let touch = match note {
                    NoteText::Simple(_) => continue,
                    NoteText::ByPlatform { touch, .. } | NoteText::TwoPlayer { touch, .. } => touch
                        .as_deref()
                        .expect("shipped control note needs touch text"),
                };
                device_notes += 1;
                assert!(
                    !touch.trim().is_empty(),
                    "{} has empty touch help",
                    level.name
                );
                for count in 0..=2 {
                    assert_eq!(note.resolve(InputHints::Touch, count), touch);
                }
                for physical_binding in [
                    "wasd",
                    "shift",
                    "joystick",
                    "controller",
                    "rb/r1",
                    "arrow keys",
                ] {
                    assert!(
                        !touch.to_lowercase().contains(physical_binding),
                        "{} still shows {physical_binding} in touch help",
                        level.name
                    );
                }
            }
        }
        assert!(device_notes > 0);
    }
}
