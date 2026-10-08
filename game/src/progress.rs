use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::storage;

static ALL_LEVELS_UNLOCKED: AtomicBool = AtomicBool::new(false);

/// Debug hook (callable from the browser console): treat every level as
/// completed, so the whole world map is open.
#[unsafe(no_mangle)]
pub extern "C" fn unlock_all_levels() {
    ALL_LEVELS_UNLOCKED.store(true, Ordering::Relaxed);
}

/// Which levels the player has completed, persisted across sessions.
///
/// Levels are stored by their base name (without directory), matching the
/// save format used before the world map existed.
#[derive(Clone, Default)]
pub(crate) struct Progress {
    completed: HashSet<String>,
}

impl Progress {
    pub(crate) fn load() -> Self {
        Self {
            completed: storage::load_completed_levels(),
        }
    }

    pub(crate) fn is_completed(&self, level: &str) -> bool {
        ALL_LEVELS_UNLOCKED.load(Ordering::Relaxed)
            || self.completed.contains(storage::strip_path_prefix(level))
    }

    /// Mark a level completed and save.
    pub(crate) fn complete(&mut self, level: &str) {
        self.completed
            .insert(storage::strip_path_prefix(level).to_string());
        storage::save_completed_levels(&self.completed);
    }

    /// Merge in imported progress and save.
    pub(crate) fn import(&mut self, levels: HashSet<String>) {
        self.completed.extend(levels);
        storage::save_completed_levels(&self.completed);
    }

    /// The progress as a copyable string, for moving it between devices.
    pub(crate) fn export(&self) -> String {
        storage::progress::encode(&self.completed)
    }
}

#[cfg(test)]
impl Progress {
    /// Progress with the given levels completed, without touching storage.
    pub(crate) fn with_completed<'a>(levels: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            completed: levels
                .into_iter()
                .map(|l| storage::strip_path_prefix(l).to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_levels_are_remembered_by_base_name() {
        let mut progress = Progress::load();
        progress.complete("cyborg_rats/fakeout");
        let reloaded = Progress::load();
        assert!(reloaded.is_completed("cyborg_rats/fakeout"));
        assert!(!reloaded.is_completed("cyborg_rats/stalemate"));
    }

    #[test]
    fn exported_progress_imports_elsewhere() {
        let mut here = Progress::with_completed(["rats", "more_rats"]);
        here.complete("webs");
        let mut there = Progress::default();
        there.import(storage::progress::decode(&here.export()).unwrap());
        for level in ["rats", "more_rats", "webs"] {
            assert!(there.is_completed(level), "{level}");
        }
    }
}
