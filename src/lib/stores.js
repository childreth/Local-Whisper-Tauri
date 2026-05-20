import { writable } from 'svelte/store';

// 'loading-model' | 'mic-permission' | 'idle' | 'recording' | 'processing' | 'error'
export const appState = writable('loading-model');

// Array of { id: number, text: string, pending: boolean }
export const transcript = writable([]);

// 0..1 RMS for the meter (throttled by AudioWorklet cadence ~3ms but rendered passively)
export const micLevel = writable(0);

// { downloaded: number, total: number } | null
export const modelProgress = writable(null);

// { kind: string, message: string } | null
export const lastError = writable(null);

// User-tunable audio preferences. Persisted to localStorage — these are a
// pure frontend audio-pipeline concern, so they don't need a backend command
// the way the global hotkey does.
export const DEFAULT_PREFERENCES = {
  silenceThreshold: 0.02, // RMS below this counts as silence
  silenceDurationMs: 800, // silent gap that ends an utterance and flushes it
};

const PREFS_KEY = 'localwhisper.preferences';

function loadPreferences() {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (raw) return { ...DEFAULT_PREFERENCES, ...JSON.parse(raw) };
  } catch {}
  return { ...DEFAULT_PREFERENCES };
}

export const preferences = writable(loadPreferences());

preferences.subscribe((value) => {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(value));
  } catch {}
});
