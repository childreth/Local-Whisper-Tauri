mod error;
#[cfg(target_os = "macos")]
mod fnhotkey;
mod model;
mod paste;
mod settings;
mod whisper;

use std::str::FromStr;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

pub use error::TranscribeError;
use whisper::WhisperEngine;

/// App-wide shared state. The WhisperEngine holds an Arc<WhisperContext> so it's
/// cheap to clone for spawn_blocking work.
pub struct AppState {
    whisper: Mutex<Option<WhisperEngine>>,
    active_model: Mutex<String>,
    // The currently-registered global shortcut. The handler reads from this so
    // the user can rebind without restarting the app.
    hotkey: Mutex<Shortcut>,
    // True when we're using the fn+Shift CGEventTap instead of the standard
    // global-shortcut plugin. macOS only; always false on other platforms.
    fn_hotkey_enabled: Mutex<bool>,
    #[cfg(target_os = "macos")]
    fn_hotkey: Mutex<Option<fnhotkey::FnHotkey>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Bootstrap hotkey from the default; settings file is loaded in setup()
    // once we have an AppHandle to find the app data dir.
    let initial_hotkey =
        Shortcut::from_str(settings::DEFAULT_HOTKEY).expect("default hotkey is valid");

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    let current = app.state::<AppState>().hotkey.lock().unwrap().clone();
                    if shortcut != &current {
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
            hotkey: Mutex::new(initial_hotkey),
            fn_hotkey_enabled: Mutex::new(false),
            #[cfg(target_os = "macos")]
            fn_hotkey: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            transcribe,
            get_model_status,
            download_model,
            paste_text,
            set_indicator_visible,
            list_models,
            set_active_model,
            get_hotkey,
            set_hotkey,
            get_fn_hotkey_enabled,
            set_fn_hotkey_enabled,
        ])
        .setup(move |app| {
            // Set up the menu bar item (Tray)
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Open Local Whisper", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // If the model is already downloaded, load it in the background so
            // the first transcribe call doesn't pay the cold-start cost.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = try_preload(handle).await {
                    eprintln!("model preload skipped: {e}");
                }
            });

            // Load user settings and apply the stored hotkey, falling back to
            // the default if the saved string no longer parses.
            let saved = settings::load(app.handle());
            let hotkey = Shortcut::from_str(&saved.hotkey).unwrap_or(initial_hotkey);
            *app.state::<AppState>().hotkey.lock().unwrap() = hotkey;

            #[cfg(target_os = "macos")]
            if saved.use_fn_hotkey {
                *app.state::<AppState>().fn_hotkey_enabled.lock().unwrap() = true;
                *app.state::<AppState>().fn_hotkey.lock().unwrap() =
                    Some(fnhotkey::FnHotkey::start(app.handle().clone()));
            } else if let Err(e) = app.global_shortcut().register(hotkey) {
                eprintln!("global hotkey registration failed: {e}");
            }
            #[cfg(not(target_os = "macos"))]
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
    if pcm.len() % 4 != 0 {
        return Err(TranscribeError::Other(
            "PCM byte length must be a multiple of 4 (f32 samples)".into(),
        ));
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
    let text = tokio::task::spawn_blocking(move || {
        // ⚡ Bolt: Zero-copy IPC buffer optimization.
        // We moved `pcm` into the spawn_blocking closure. If the incoming Vec<u8> memory
        // buffer is correctly aligned to 4 bytes (which it almost always is), we borrow it
        // directly as a &[f32] slice. This prevents allocating a duplicate Vec<f32> and
        // eliminates O(N) memory copying of the audio buffer before inference.
        let n = pcm.len() / 4;
        if pcm.as_ptr().align_offset(std::mem::align_of::<f32>()) == 0 {
            let samples = unsafe { std::slice::from_raw_parts(pcm.as_ptr() as *const f32, n) };
            whisper::run_inference(&ctx, samples)
        } else {
            let mut samples = Vec::with_capacity(n);
            for chunk in pcm.chunks_exact(4) {
                samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
            whisper::run_inference(&ctx, &samples)
        }
    })
    .await
    .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;

    Ok(text)
}

#[tauri::command]
async fn paste_text(text: String) -> Result<(), TranscribeError> {
    paste::paste_text(text).await
}

#[tauri::command]
fn get_hotkey(state: tauri::State<'_, AppState>) -> String {
    state.hotkey.lock().unwrap().to_string()
}

#[tauri::command]
fn set_hotkey(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    accel: String,
) -> Result<(), TranscribeError> {
    let new = Shortcut::from_str(&accel)
        .map_err(|e| TranscribeError::Other(format!("invalid shortcut '{accel}': {e}")))?;

    let fn_enabled = *state.fn_hotkey_enabled.lock().unwrap();
    let old = { *state.hotkey.lock().unwrap() };
    if old != new {
        // Only touch the standard shortcut registration when the fn-hotkey is
        // off. Otherwise the user is just updating the fallback for later.
        if !fn_enabled {
            let gs = app.global_shortcut();
            let _ = gs.unregister(old);
            gs.register(new)
                .map_err(|e| TranscribeError::Other(format!("register hotkey: {e}")))?;
        }
        *state.hotkey.lock().unwrap() = new;
    }

    settings::save(
        &app,
        &settings::Settings {
            hotkey: accel,
            use_fn_hotkey: fn_enabled,
        },
    )?;
    Ok(())
}

#[tauri::command]
fn get_fn_hotkey_enabled(state: tauri::State<'_, AppState>) -> bool {
    *state.fn_hotkey_enabled.lock().unwrap()
}

#[tauri::command]
#[allow(unused_variables)]
fn set_fn_hotkey_enabled(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), TranscribeError> {
    #[cfg(not(target_os = "macos"))]
    if enabled {
        return Err(TranscribeError::Other(
            "fn hotkey is macOS-only".into(),
        ));
    }

    let was = *state.fn_hotkey_enabled.lock().unwrap();
    if was == enabled {
        return Ok(());
    }

    let current = { *state.hotkey.lock().unwrap() };

    #[cfg(target_os = "macos")]
    {
        if enabled {
            // Tear down the standard shortcut so we don't have two systems
            // listening at once, then start the event tap.
            let _ = app.global_shortcut().unregister(current);
            *state.fn_hotkey.lock().unwrap() =
                Some(fnhotkey::FnHotkey::start(app.clone()));
        } else {
            // Stop the tap, then restore the configured standard shortcut.
            if let Some(h) = state.fn_hotkey.lock().unwrap().take() {
                h.stop();
            }
            app.global_shortcut()
                .register(current)
                .map_err(|e| TranscribeError::Other(format!("register hotkey: {e}")))?;
        }
    }

    *state.fn_hotkey_enabled.lock().unwrap() = enabled;

    let accel = current.to_string();
    settings::save(
        &app,
        &settings::Settings {
            hotkey: accel,
            use_fn_hotkey: enabled,
        },
    )?;
    Ok(())
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
            let x = (mon_size.width as f64) / scale / 2.0 - 160.0;
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
