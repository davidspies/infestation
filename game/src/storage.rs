use std::collections::HashSet;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use serde::de::DeserializeOwned;

macro_rules! warn_err {
    ($expr:expr, $($arg:tt)+) => {
        $expr.map_err(|e| log::warn!($($arg)+, e)).ok()
    };
}

const COMPLETED_LEVELS: &str = "completed_levels";

pub(crate) fn load_completed_levels() -> HashSet<String> {
    load::<HashSet<String>>(COMPLETED_LEVELS)
        .unwrap_or_default()
        .into_iter()
        .map(|s| strip_path_prefix(&s).to_string())
        .collect()
}

pub(crate) fn save_completed_levels(completed: &HashSet<String>) {
    save(COMPLETED_LEVELS, completed);
}

/// Load a saved value, if there is one (and it parses).
pub(crate) fn load<T: DeserializeOwned>(name: &str) -> Option<T> {
    let json = backend::load(name)?;
    warn_err!(
        serde_json::from_str(&json),
        "Failed to parse saved {}: {}",
        name
    )
}

pub(crate) fn save<T: Serialize>(name: &str, value: &T) {
    let json = serde_json::to_string(value).expect("saved values should serialize");
    backend::save(name, &json);
}

fn encode_progress(completed: &HashSet<String>) -> String {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut sorted: Vec<&str> = completed.iter().map(String::as_str).collect();
    sorted.sort();
    let json = serde_json::to_string(&sorted).expect("completed levels should serialize");

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(json.as_bytes())
        .expect("zlib write should succeed");
    let compressed = encoder.finish().expect("zlib finish should succeed");

    STANDARD.encode(&compressed)
}

fn decode_progress(encoded: &str) -> Result<HashSet<String>, String> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let compressed = STANDARD
        .decode(encoded)
        .map_err(|e| format!("invalid base64: {e}"))?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut json_str = String::new();
    decoder
        .read_to_string(&mut json_str)
        .map_err(|e| format!("invalid zlib data: {e}"))?;
    let levels: Vec<String> =
        serde_json::from_str(&json_str).map_err(|e| format!("invalid JSON: {e}"))?;
    Ok(levels.into_iter().collect())
}

pub(crate) fn strip_path_prefix(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

#[cfg(all(target_arch = "wasm32", not(test)))]
mod backend {
    fn key(name: &str) -> String {
        format!("infestation_{name}")
    }

    pub(super) fn save(name: &str, json: &str) {
        quad_storage::STORAGE.lock().unwrap().set(&key(name), json);
    }

    pub(super) fn load(name: &str) -> Option<String> {
        quad_storage::STORAGE.lock().unwrap().get(&key(name))
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
mod backend {
    use std::fs::{create_dir_all, read_to_string, write};
    use std::path::PathBuf;

    pub(super) fn save(name: &str, json: &str) {
        if let Some(path) = save_path(name) {
            if let Some(parent) = path.parent() {
                warn_err!(
                    create_dir_all(parent),
                    "Failed to create {}: {}",
                    parent.display()
                );
            }
            warn_err!(write(&path, json), "Failed to write {}: {}", path.display());
        }
    }

    pub(super) fn load(name: &str) -> Option<String> {
        let path = save_path(name)?;
        if !path.exists() {
            return None;
        }
        warn_err!(
            read_to_string(&path),
            "Failed to read {}: {}",
            path.display()
        )
    }

    fn save_path(name: &str) -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "dspyz", "InfestationGame")?;
        Some(dirs.data_dir().join(format!("{name}.json")))
    }
}

/// Tests keep saves in memory, so they can never touch the player's real
/// save files.
#[cfg(test)]
mod backend {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static SAVES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    }

    pub(super) fn save(name: &str, json: &str) {
        SAVES.with(|saves| {
            saves
                .borrow_mut()
                .insert(name.to_string(), json.to_string())
        });
    }

    pub(super) fn load(name: &str) -> Option<String> {
        SAVES.with(|saves| saves.borrow().get(name).cloned())
    }
}

/// Clipboard-based progress export/import.
pub(crate) mod progress {
    use super::*;

    pub(crate) fn encode(completed: &HashSet<String>) -> String {
        encode_progress(completed)
    }

    pub(crate) fn decode(encoded: &str) -> Result<HashSet<String>, String> {
        decode_progress(encoded)
    }

    pub(crate) fn copy_to_clipboard(text: &str) {
        clipboard::write(text);
    }

    pub(crate) fn start_import() {
        clipboard::start_read();
    }

    pub(crate) fn poll_import() -> Option<HashSet<String>> {
        let encoded = clipboard::poll_read()?;
        match decode(&encoded) {
            Ok(levels) => Some(levels),
            Err(e) => {
                log::warn!("Failed to decode imported progress: {}", e);
                None
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    mod clipboard {
        const CLIPBOARD_KEY: &str = "infestation_clipboard_data";

        unsafe extern "C" {
            fn sapp_clipboard_write(buf: *const u8, len: u32);
            fn sapp_clipboard_read();
        }

        pub(super) fn write(text: &str) {
            unsafe { sapp_clipboard_write(text.as_ptr(), text.len() as u32) };
        }

        pub(super) fn start_read() {
            unsafe { sapp_clipboard_read() };
        }

        pub(super) fn poll_read() -> Option<String> {
            let storage = quad_storage::STORAGE.lock().unwrap();
            let text = storage.get(CLIPBOARD_KEY)?;
            drop(storage);
            quad_storage::STORAGE.lock().unwrap().remove(CLIPBOARD_KEY);
            Some(text)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod clipboard {
        use std::sync::Mutex;

        static PENDING: Mutex<Option<String>> = Mutex::new(None);

        pub(super) fn write(text: &str) {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }

        pub(super) fn start_read() {
            let result = arboard::Clipboard::new()
                .and_then(|mut c| c.get_text())
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string());
            *PENDING.lock().unwrap() = result;
        }

        pub(super) fn poll_read() -> Option<String> {
            PENDING.lock().unwrap().take()
        }
    }
}
