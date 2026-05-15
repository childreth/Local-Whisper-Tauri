use std::path::Path;
use std::sync::Arc;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::error::TranscribeError;

/// In-process whisper.cpp engine. Holds an Arc<WhisperContext> so it can be
/// cheaply shared with worker threads via spawn_blocking.
pub struct WhisperEngine {
    ctx: Arc<WhisperContext>,
}

impl WhisperEngine {
    pub fn load(path: &Path) -> Result<Self, TranscribeError> {
        let path_str = path
            .to_str()
            .ok_or_else(|| TranscribeError::Other("non-utf8 model path".into()))?;
        let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
            .map_err(|e| TranscribeError::WhisperFailed(format!("load: {e}")))?;
        Ok(Self {
            ctx: Arc::new(ctx),
        })
    }

    pub fn context(&self) -> Arc<WhisperContext> {
        self.ctx.clone()
    }
}

/// Blocking inference over a slice of f32 PCM samples at 16kHz mono.
/// Returns concatenated segment text.
pub fn run_inference(ctx: &WhisperContext, samples: &[f32]) -> Result<String, TranscribeError> {
    // Sanity check: less than ~100ms of audio isn't worth running through whisper.
    if samples.len() < 1600 {
        return Err(TranscribeError::AudioTooShort(samples.len()));
    }

    let mut state = ctx
        .create_state()
        .map_err(|e| TranscribeError::WhisperFailed(format!("create_state: {e}")))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    params.set_n_threads(threads);

    state
        .full(params, samples)
        .map_err(|e| TranscribeError::WhisperFailed(format!("full: {e}")))?;

    // whisper-rs 0.16: full_n_segments returns i32 directly (no Result),
    // and per-segment text is read via get_segment(i).to_str().
    let n = state.full_n_segments();

    let mut out = String::new();
    for i in 0..n {
        let segment = state
            .get_segment(i)
            .ok_or_else(|| TranscribeError::WhisperFailed(format!("missing segment {i}")))?;
        let text = segment
            .to_str()
            .map_err(|e| TranscribeError::WhisperFailed(format!("segment_text: {e}")))?;
        out.push_str(text);
    }
    Ok(out)
}
