mod error;
mod model;
mod whisper;

use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub use error::TranscribeError;
use whisper::WhisperEngine;

/// App-wide shared state. The WhisperEngine holds an Arc<WhisperContext> so it's
/// cheap to clone for spawn_blocking work.
pub struct AppState {
    whisper: Mutex<Option<WhisperEngine>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            whisper: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            transcribe,
            get_model_status,
            download_model,
        ])
        .setup(|app| {
            // If the model is already downloaded, load it in the background so
            // the first transcribe call doesn't pay the cold-start cost.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = try_preload(handle).await {
                    eprintln!("model preload skipped: {e}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn try_preload(handle: AppHandle) -> Result<(), TranscribeError> {
    let path = model::model_path(&handle)?;
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
    let path = model::model_path(&app)?;
    Ok(if path.exists() { "ready" } else { "missing" })
}

#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), TranscribeError> {
    let path = model::ensure_model(app.clone()).await?;
    let engine = tokio::task::spawn_blocking(move || WhisperEngine::load(&path))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))??;
    *state.whisper.lock().unwrap() = Some(engine);
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
