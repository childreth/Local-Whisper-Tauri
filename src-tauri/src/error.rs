use serde::Serialize;
use thiserror::Error;

/// Errors surfaced to the frontend. Implements Serialize as a string so Tauri
/// can ship it across the IPC boundary.
#[derive(Debug, Error)]
pub enum TranscribeError {
    #[error("model not loaded")]
    ModelNotLoaded,

    #[error("audio too short ({0} samples)")]
    AudioTooShort(usize),

    #[error("whisper failed: {0}")]
    WhisperFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("download failed: {0}")]
    Download(String),

    #[error("hash mismatch (expected {expected}, got {actual})")]
    HashMismatch { expected: String, actual: String },

    #[error("{0}")]
    Other(String),
}

impl Serialize for TranscribeError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
