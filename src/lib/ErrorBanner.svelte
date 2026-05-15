<script>
  import { lastError } from './stores.js';

  const LABELS = {
    init: 'Initialization error',
    'model-load': 'Model load failed',
    'mic-permission': 'Microphone unavailable',
    transcribe: 'Transcription failed',
  };

  $: label = $lastError ? LABELS[$lastError.kind] ?? 'Error' : '';

  function dismiss() {
    lastError.set(null);
  }
</script>

{#if $lastError}
  <div class="banner" role="alert">
    <span class="icon" aria-hidden="true">⚠️</span>
    <div class="content">
      <strong>{label}</strong>
      <p>{$lastError.message}</p>
    </div>
    <button class="dismiss" on:click={dismiss} aria-label="Dismiss">✕</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    background: rgba(255, 82, 82, 0.1);
    border: 1px solid rgba(255, 82, 82, 0.35);
    border-radius: var(--radius);
    margin-bottom: 1rem;
  }
  .icon {
    font-size: 1.125rem;
    line-height: 1.2;
  }
  .content {
    flex: 1;
  }
  .content strong {
    display: block;
    font-size: 0.875rem;
    margin-bottom: 0.125rem;
  }
  .content p {
    margin: 0;
    color: var(--text-dim);
    font-size: 0.8125rem;
    line-height: 1.5;
  }
  .dismiss {
    background: transparent;
    color: var(--text-dim);
    border: none;
    padding: 0 0.25rem;
    font-size: 1rem;
    line-height: 1;
  }
  .dismiss:hover {
    color: var(--text);
  }
</style>
