<script>
  import { transcript } from './stores.js';

  let copied = false;

  async function copyAll() {
    const text = $transcript
      .filter((s) => !s.pending)
      .map((s) => s.text)
      .join('\n');
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      // Clipboard may be blocked in some Tauri configs; silent fail is fine.
    }
  }

  function clearAll() {
    transcript.set([]);
  }
</script>

<div class="container">
  <div class="header">
    <h2>Transcript</h2>
    <div class="actions">
      <button on:click={copyAll} disabled={$transcript.length === 0}>
        {copied ? '✓ Copied' : '📋 Copy'}
      </button>
      <button on:click={clearAll} disabled={$transcript.length === 0}>
        🗑️ Clear
      </button>
    </div>
  </div>

  <div class="segments">
    {#if $transcript.length === 0}
      <p class="empty">Press Record and start talking. Transcription appears here. 🎤</p>
    {:else}
      {#each $transcript as seg (seg.id)}
        <div class="segment" class:pending={seg.pending}>
          {seg.text}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .container {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 1rem 1.25rem;
    min-height: 220px;
    display: flex;
    flex-direction: column;
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }
  h2 {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-dim);
    margin: 0;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .actions button {
    background: var(--surface-2);
    color: var(--text);
    border: 1px solid var(--border);
    padding: 0.375rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8125rem;
    transition: background 120ms ease;
  }
  .actions button:hover:not(:disabled) {
    background: var(--border);
  }
  .actions button:disabled {
    opacity: 0.4;
  }
  .segments {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    flex: 1;
  }
  .segment {
    padding: 0.625rem 0.875rem;
    background: var(--surface-2);
    border-radius: 6px;
    line-height: 1.5;
    font-size: 0.9375rem;
    word-wrap: break-word;
  }
  .segment.pending {
    color: var(--text-dim);
    font-style: italic;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.875rem;
    text-align: center;
    padding: 2.5rem 0;
    margin: 0;
  }
</style>
