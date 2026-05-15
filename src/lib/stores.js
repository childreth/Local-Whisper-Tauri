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
