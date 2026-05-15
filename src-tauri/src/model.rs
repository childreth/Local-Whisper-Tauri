use std::path::PathBuf;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

use crate::error::TranscribeError;

const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin";
const MODEL_FILE: &str = "ggml-small.en.bin";

/// If you want strict integrity checking, paste the expected hex digest here.
/// Compute it once locally with `shasum -a 256 ggml-small.en.bin`.
/// Set to `None` to skip verification (you'll still see file-not-found errors
/// if the download is truncated).
const EXPECTED_SHA256: Option<&str> = None;

/// Throttle progress events to at most this often. The download is fast enough
/// that emitting on every chunk floods the IPC and stalls the UI.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

pub fn model_path(app: &AppHandle) -> Result<PathBuf, TranscribeError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| TranscribeError::Other(format!("app_data_dir: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(MODEL_FILE))
}

/// Returns the final model path, downloading it first if needed.
/// Emits `model:progress` events with `{ downloaded, total }` during download.
pub async fn ensure_model(app: AppHandle) -> Result<PathBuf, TranscribeError> {
    let path = model_path(&app)?;
    if path.exists() {
        // If the file is suspiciously small, treat it as broken and re-download.
        let meta = tokio::fs::metadata(&path).await?;
        if meta.len() > 10_000_000 {
            return Ok(path);
        }
        let _ = tokio::fs::remove_file(&path).await;
    }

    let tmp_path = path.with_extension("bin.downloading");
    let _ = tokio::fs::remove_file(&tmp_path).await; // ignore if not present

    let resp = reqwest::get(MODEL_URL)
        .await
        .map_err(|e| TranscribeError::Download(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(TranscribeError::Download(format!(
            "HTTP {}",
            resp.status()
        )));
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
                    "downloaded": downloaded,
                    "total": total,
                }),
            );
            last_emit = Instant::now();
        }
    }
    file.flush().await?;
    drop(file);

    // Final progress event so the UI shows 100%.
    let _ = app.emit(
        "model:progress",
        serde_json::json!({
            "downloaded": downloaded,
            "total": total.max(downloaded),
        }),
    );

    // Optional hash verification.
    if let Some(expected) = EXPECTED_SHA256 {
        let actual = hex_lower(&hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(TranscribeError::HashMismatch {
                expected: expected.to_string(),
                actual,
            });
        }
    }

    // Atomically replace the final path.
    tokio::fs::rename(&tmp_path, &path).await?;
    Ok(path)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}
