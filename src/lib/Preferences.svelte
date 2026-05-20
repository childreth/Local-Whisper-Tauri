<script>
  import { preferences, DEFAULT_PREFERENCES } from './stores.js';

  function update(key, value) {
    preferences.update((p) => ({ ...p, [key]: value }));
  }

  function reset() {
    preferences.set({ ...DEFAULT_PREFERENCES });
  }

  $: isDefault =
    $preferences.silenceThreshold === DEFAULT_PREFERENCES.silenceThreshold &&
    $preferences.silenceDurationMs === DEFAULT_PREFERENCES.silenceDurationMs;
</script>

<div class="card">
  <div class="card-header">
    <div class="text">
      <div class="title">Preferences</div>
      <div class="desc">Tune how the app decides when you've stopped speaking.</div>
    </div>
    <button class="reset" on:click={reset} disabled={isDefault}>Reset</button>
  </div>

  <div class="setting">
    <div class="setting-label">
      <span class="name">Silence threshold</span>
      <span class="value">{$preferences.silenceThreshold.toFixed(3)}</span>
    </div>
    <input
      type="range"
      min="0.005"
      max="0.1"
      step="0.005"
      value={$preferences.silenceThreshold}
      on:input={(e) => update('silenceThreshold', parseFloat(e.target.value))}
    />
    <p class="desc">
      Audio quieter than this RMS level counts as silence. Lower it if quiet speech
      gets dropped; raise it if background noise keeps a recording from settling.
    </p>
  </div>

  <div class="setting">
    <div class="setting-label">
      <span class="name">Silence pause</span>
      <span class="value">{$preferences.silenceDurationMs} ms</span>
    </div>
    <input
      type="range"
      min="300"
      max="2000"
      step="100"
      value={$preferences.silenceDurationMs}
      on:input={(e) => update('silenceDurationMs', parseInt(e.target.value, 10))}
    />
    <p class="desc">
      How long a silent gap must last before the current utterance is sent for
      transcription.
    </p>
  </div>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem 1.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 0.9375rem;
    font-weight: 600;
  }
  .desc {
    color: var(--text-dim);
    font-size: 0.8125rem;
    margin: 0.125rem 0 0;
  }
  .reset {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 0.75rem;
    padding: 0.25rem 0.625rem;
    border-radius: 6px;
  }
  .reset:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--text-dim);
  }
  .reset:disabled {
    opacity: 0.4;
  }
  .setting {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    padding-top: 0.875rem;
    border-top: 1px solid var(--border);
  }
  .setting-label {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .name {
    font-size: 0.875rem;
    font-weight: 500;
  }
  .value {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8125rem;
    color: var(--text-dim);
  }
  input[type='range'] {
    width: 100%;
    accent-color: var(--accent);
    cursor: pointer;
  }
</style>
