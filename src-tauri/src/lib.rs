mod error;
mod model;
mod paste;
mod whisper;

use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub use error::TranscribeError;
use whisper::WhisperEngine;

/// App-wide shared state. The WhisperEngine holds an Arc<WhisperContext> so it's
/// cheap to clone for spawn_blocking work.
pub struct AppState {
    whisper: Mutex<Option<WhisperEngine>>,
    active_model: Mutex<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Ctrl+Opt+Space — hold to record, release to stop.
    let hotkey = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space);

    let hotkey_for_handler = hotkey;
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut != &hotkey_for_handler {
                        return;
                    }
                    let name = match event.state() {
                        ShortcutState::Pressed => "hotkey:down",
                        ShortcutState::Released => "hotkey:up",
                    };
                    if let Err(e) = app.emit(name, ()) {
                        eprintln!("emit {name} failed: {e}");
                    }
                })
                .build(),
        )
        .manage(AppState {
            whisper: Mutex::new(None),
            active_model: Mutex::new(model::DEFAULT_MODEL_ID.to_string()),
        })
        .invoke_handler(tauri::generate_handler![
            transcribe,
            get_model_status,
            download_model,
            paste_text,
            set_indicator_visible,
            list_models,
            set_active_model,
        ])
        .setup(move |app| {
            // If the model is already downloaded, load it in the background so
            // the first transcribe call doesn't pay the cold-start cost.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = try_preload(handle).await {
                    eprintln!("model preload skipped: {e}");
                }
            });

            if let Err(e) = app.global_shortcut().register(hotkey) {
                eprintln!("global hotkey registration failed: {e}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn try_preload(handle: AppHandle) -> Result<(), TranscribeError> {
    let id = {
        let state = handle.state::<AppState>();
        let id = state.active_model.lock().unwrap().clone();
        id
    };
    let path = model::model_path(&handle, &id)?;
    if !path.exists() {
        return Ok(());
    }
    let engine = tokio::task::spawn_blocking(move || WhisperEngine::load(&path))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;
    let state = handle.state::<AppState>();
    *state.whisper.lock().unwrap() = Some(engine);
    Ok(())
}

#[tauri::command]
async fn get_model_status(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<&'static str, TranscribeError> {
    if state.whisper.lock().unwrap().is_some() {
        return Ok("ready");
    }
    let id = state.active_model.lock().unwrap().clone();
    let path = model::model_path(&app, &id)?;
    Ok(if path.exists() { "ready" } else { "missing" })
}

#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), TranscribeError> {
    let id = state.active_model.lock().unwrap().clone();
    let path = model::ensure_model(app.clone(), id).await?;
    let engine = tokio::task::spawn_blocking(move || WhisperEngine::load(&path))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;
    *state.whisper.lock().unwrap() = Some(engine);
    Ok(())
}

#[derive(serde::Serialize)]
struct ModelEntry {
    id: &'static str,
    label: &'static str,
    size_mb: u64,
    present: bool,
    active: bool,
}

#[tauri::command]
fn list_models(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ModelEntry>, TranscribeError> {
    let active = state.active_model.lock().unwrap().clone();
    let mut out = Vec::with_capacity(model::MODELS.len());
    for m in model::MODELS {
        let path = model::model_path(&app, m.id)?;
        out.push(ModelEntry {
            id: m.id,
            label: m.label,
            size_mb: m.size_mb,
            present: path.exists(),
            active: m.id == active,
        });
    }
    Ok(out)
}

/// Switch the active model. Downloads if missing, then loads. No-op if
/// `id` is already the active loaded model.
#[tauri::command]
async fn set_active_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), TranscribeError> {
    model::find_model(&id)?; // validate

    {
        let active = state.active_model.lock().unwrap();
        if *active == id && state.whisper.lock().unwrap().is_some() {
            return Ok(());
        }
    }

    // Drop the currently loaded engine before loading a new one to free memory.
    *state.whisper.lock().unwrap() = None;

    let path = model::ensure_model(app.clone(), id.clone()).await?;
    let engine = tokio::task::spawn_blocking(move || WhisperEngine::load(&path))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;

    *state.whisper.lock().unwrap() = Some(engine);
    *state.active_model.lock().unwrap() = id;
    Ok(())
}

#[tauri::command]
async fn transcribe(
    pcm: Vec<u8>,
    state: tauri::State<'_, AppState>,
) -> Result<String, TranscribeError> {
    if pcm.is_empty() {
        return Err(TranscribeError::AudioTooShort(0));
    }
    if pcm.len() % 2 != 0 {
        return Err(TranscribeError::Other(
            "PCM byte length must be even (Int16 samples)".into(),
        ));
    }

    // Decode Int16 LE bytes to f32 in [-1.0, 1.0] — whisper-rs wants f32.
    let n = pcm.len() / 2;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let s = i16::from_le_bytes([pcm[i * 2], pcm[i * 2 + 1]]);
        samples.push(s as f32 / 32768.0);
    }

    // Grab the WhisperContext (Arc) while holding the lock briefly.
    let ctx = {
        let guard = state.whisper.lock().unwrap();
        let engine = guard
            .as_ref()
            .ok_or(TranscribeError::ModelNotLoaded)?;
        engine.context()
    };

    // Run inference off the async runtime — whisper is blocking + CPU-heavy.
    let text = tokio::task::spawn_blocking(move || whisper::run_inference(&ctx, &samples))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;

    Ok(text)
}

#[tauri::command]
async fn paste_text(text: String) -> Result<(), TranscribeError> {
    paste::paste_text(text).await
}

#[tauri::command]
fn set_indicator_visible(app: AppHandle, visible: bool) -> Result<(), TranscribeError> {
    let win = app
        .get_webview_window("indicator")
        .ok_or_else(|| TranscribeError::Other("indicator window missing".into()))?;
    if visible {
        // Position above the notch / top-center each time we show, in case the
        // user moved displays. Width is fixed in tauri.conf.json (200px).
        if let Ok(Some(monitor)) = win.current_monitor() {
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();
            let x = (mon_size.width as f64) / scale / 2.0 - 100.0;
            let y = 8.0;
            let _ = win.set_position(tauri::LogicalPosition::new(x, y));
        }
        win.show()
            .map_err(|e| TranscribeError::Other(format!("show: {e}")))?;
    } else {
        win.hide()
            .map_err(|e| TranscribeError::Other(format!("hide: {e}")))?;
    }
    Ok(())
}
