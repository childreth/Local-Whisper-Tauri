<script>
  import { onMount, onDestroy } from 'svelte';
  import { appState, modelProgress, lastError } from './stores.js';
  import { downloadModel, onModelProgress } from './tauri-bridge.js';

  let unlisten = null;

  onMount(async () => {
    try {
      unlisten = await onModelProgress((payload) => modelProgress.set(payload));
      await downloadModel();
      modelProgress.set(null);
      appState.set('idle');
    } catch (e) {
      lastError.set({ kind: 'model-load', message: String(e) });
      appState.set('error');
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  $: pct = $modelProgress
    ? Math.round(($modelProgress.downloaded / $modelProgress.total) * 100)
    : 0;
  $: mb = $modelProgress
    ? ($modelProgress.downloaded / (1024 * 1024)).toFixed(1)
    : '0.0';
  $: totalMb = $modelProgress
    ? ($modelProgress.total / (1024 * 1024)).toFixed(0)
    : '~150';
</script>

<div class="container">
  <h2>📥 Setting things up</h2>
  <p class="hint">
    Downloading the Whisper small.en model (~150&nbsp;MB) so transcription can run locally.
    This happens once.
  </p>
  <div class="bar">
    <div class="fill" style="width: {pct}%"></div>
  </div>
  <p class="status">{mb} / {totalMb} MB · {pct}%</p>
</div>

<style>
  .container {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 2rem;
    text-align: center;
  }
  h2 {
    margin: 0 0 0.5rem;
    font-size: 1.0625rem;
    font-weight: 600;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.875rem;
    margin: 0 0 1.5rem;
    line-height: 1.5;
  }
  .bar {
    height: 8px;
    background: var(--surface-2);
    border-radius: 4px;
    overflow: hidden;
    margin: 1rem 0 0.5rem;
  }
  .fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-hover));
    transition: width 200ms ease-out;
  }
  .status {
    color: var(--text-dim);
    font-size: 0.8125rem;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
</style>
