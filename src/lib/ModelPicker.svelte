<script>
  import { onMount } from 'svelte';
  import { listModels, setActiveModel, onModelProgress } from './tauri-bridge.js';
  import { lastError } from './stores.js';

  let models = [];
  let activeId = '';
  let busy = false;
  let progress = null; // { id, downloaded, total } during download

  async function refresh() {
    try {
      models = await listModels();
      activeId = models.find((m) => m.active)?.id ?? '';
    } catch (e) {
      lastError.set({ kind: 'models', message: String(e) });
    }
  }

  async function handleChange(e) {
    const id = e.target.value;
    if (busy) {
      // Snap the <select> back; we won't service this change.
      e.target.value = activeId;
      return;
    }
    busy = true;
    progress = null;
    try {
      await setActiveModel(id);
      await refresh();
    } catch (err) {
      lastError.set({ kind: 'model-switch', message: String(err) });
      await refresh();
    } finally {
      busy = false;
      progress = null;
    }
  }

  onMount(async () => {
    await refresh();
    await onModelProgress((p) => {
      progress = p;
    });
  });

  $: pct = progress && progress.total > 0
    ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
    : null;
</script>

<div class="picker">
  <label for="model-select">Model</label>
  <select id="model-select" value={activeId} on:change={handleChange} disabled={busy}>
    {#each models as m}
      <option value={m.id}>
        {m.label} {m.present ? '' : `· ${m.size_mb} MB download`}
      </option>
    {/each}
  </select>
  {#if busy}
    <span class="status">
      {#if pct !== null}Downloading {pct}%{:else}Loading…{/if}
    </span>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.875rem;
  }
  label {
    color: var(--text-dim);
  }
  select {
    background: var(--surface-2);
    color: inherit;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.25rem 0.5rem;
    font: inherit;
  }
  select:disabled {
    opacity: 0.6;
  }
  .status {
    color: var(--text-dim);
    font-size: 0.8125rem;
    margin-left: auto;
  }
</style>
