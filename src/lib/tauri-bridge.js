import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/**
 * Send Int16 PCM (16kHz mono) to the Rust backend for transcription.
 * @param {Uint8Array} pcm - little-endian Int16 byte stream
 * @returns {Promise<string>} transcript text
 */
export async function transcribe(pcm) {
  // Tauri v2 IPC serializes binary as a number array. For ~1-2s of 16kHz Int16
  // (32-64KB) this is fine; for longer audio consider Channel<T> instead.
  return await invoke('transcribe', { pcm: Array.from(pcm) });
}

/**
 * Ask the backend whether the whisper model is ready, missing, or downloading.
 * @returns {Promise<'missing' | 'downloading' | 'ready'>}
 */
export async function getModelStatus() {
  return await invoke('get_model_status');
}

/**
 * Kick off (or resume) the model download. Listen for `model:progress`
 * events for progress updates, and await the returned promise for completion.
 */
export async function downloadModel() {
  return await invoke('download_model');
}

/**
 * Subscribe to model download progress events.
 * @param {(payload: { downloaded: number, total: number }) => void} cb
 * @returns {Promise<() => void>} unlisten function
 */
export async function onModelProgress(cb) {
  return await listen('model:progress', (event) => cb(event.payload));
}
