use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::TranscribeError;

pub const DEFAULT_HOTKEY: &str = "Control+Alt+Space";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub hotkey: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: DEFAULT_HOTKEY.to_string(),
        }
    }
}

fn path(app: &AppHandle) -> Result<PathBuf, TranscribeError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| TranscribeError::Other(format!("app_data_dir: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    let p = match path(app) {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Settings::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), TranscribeError> {
    let p = path(app)?;
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| TranscribeError::Other(format!("serialize settings: {e}")))?;
    std::fs::write(&p, text)?;
    Ok(())
}
