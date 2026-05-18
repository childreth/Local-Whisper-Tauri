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

/**
 * Paste text into whatever app is currently focused, via clipboard + synthetic Cmd+V.
 * The Local Whisper window must NOT be focused when this runs.
 * Requires Accessibility permission on macOS.
 * @param {string} text
 */
export async function pasteText(text) {
  return await invoke('paste_text', { text });
}

/**
 * Subscribe to global hotkey press/release events emitted from Rust.
 * Returns a combined unlisten function.
 * @param {{ onDown: () => void, onUp: () => void }} handlers
 */
export async function onHotkey({ onDown, onUp }) {
  const unDown = await listen('hotkey:down', () => onDown());
  const unUp = await listen('hotkey:up', () => onUp());
  return () => {
    unDown();
    unUp();
  };
}

/**
 * Show or hide the floating status pill window.
 * @param {boolean} visible
 */
export async function setIndicatorVisible(visible) {
  return await invoke('set_indicator_visible', { visible });
}

/**
 * @returns {Promise<Array<{id: string, label: string, size_mb: number, present: boolean, active: boolean}>>}
 */
export async function listModels() {
  return await invoke('list_models');
}

/**
 * Switch active model. Downloads first if not present. Resolves once loaded.
 * @param {string} id
 */
export async function setActiveModel(id) {
  return await invoke('set_active_model', { id });
}
