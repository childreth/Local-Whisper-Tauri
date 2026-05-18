use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::error::TranscribeError;

/// All models known to the app. The `id` is used in IPC payloads.
pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "base.en",
        label: "Base (74M)",
        file: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        size_mb: 142,
    },
    ModelInfo {
        id: "small.en",
        label: "Small (244M)",
        file: "ggml-small.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        size_mb: 466,
    },
];

pub const DEFAULT_MODEL_ID: &str = "small.en";

#[derive(Clone, Copy, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub label: &'static str,
    #[serde(skip)]
    pub file: &'static str,
    #[serde(skip)]
    pub url: &'static str,
    pub size_mb: u64,
}

pub fn find_model(id: &str) -> Result<&'static ModelInfo, TranscribeError> {
    MODELS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| TranscribeError::Other(format!("unknown model id: {id}")))
}

/// Throttle progress events to at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

pub fn model_path(app: &AppHandle, id: &str) -> Result<PathBuf, TranscribeError> {
    let info = find_model(id)?;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| TranscribeError::Other(format!("app_data_dir: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(info.file))
}

/// Downloads the chosen model if missing. Emits `model:progress` with
/// `{ id, downloaded, total }` during download.
pub async fn ensure_model(app: AppHandle, id: String) -> Result<PathBuf, TranscribeError> {
    let info = *find_model(&id)?;
    let path = model_path(&app, &id)?;

    if path.exists() {
        let meta = tokio::fs::metadata(&path).await?;
        // Sanity floor — partial downloads are smaller than any real model.
        if meta.len() > 10_000_000 {
            return Ok(path);
        }
        let _ = tokio::fs::remove_file(&path).await;
    }

    let tmp_path = path.with_extension("bin.downloading");
    let _ = tokio::fs::remove_file(&tmp_path).await;

    let resp = reqwest::get(info.url)
        .await
        .map_err(|e| TranscribeError::Download(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(TranscribeError::Download(format!("HTTP {}", resp.status())));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    let mut hasher = Sha256::new();

    let mut file = tokio::fs::File::create(&tmp_path).await?;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| TranscribeError::Download(e.to_string()))?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        if last_emit.elapsed() >= PROGRESS_INTERVAL {
            let _ = app.emit(
                "model:progress",
                serde_json::json!({
                    "id": info.id,
                    "downloaded": downloaded,
                    "total": total,
                }),
            );
            last_emit = Instant::now();
        }
    }
    file.flush().await?;
    drop(file);

    let _ = app.emit(
        "model:progress",
        serde_json::json!({
            "id": info.id,
            "downloaded": downloaded,
            "total": total.max(downloaded),
        }),
    );

    tokio::fs::rename(&tmp_path, &path).await?;
    Ok(path)
}

#[allow(dead_code)]
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
