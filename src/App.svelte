<script>
  import { onMount } from 'svelte';
  import { appState, lastError } from './lib/stores.js';
  import { getModelStatus } from './lib/tauri-bridge.js';
  import ModelLoader from './lib/ModelLoader.svelte';
  import Transcriber from './lib/Transcriber.svelte';
  import ErrorBanner from './lib/ErrorBanner.svelte';
  import PasteTester from './lib/PasteTester.svelte';

  onMount(async () => {
    try {
      const status = await getModelStatus();
      appState.set(status === 'ready' ? 'idle' : 'loading-model');
    } catch (e) {
      lastError.set({ kind: 'init', message: String(e) });
      appState.set('error');
    }
  });
</script>

<main>
  <header>
    <h1>🎙️ Local Whisper</h1>
    <p class="subtitle">Local speech-to-text. Nothing leaves your machine.</p>
  </header>

  <ErrorBanner />

  {#if $appState === 'loading-model'}
    <ModelLoader />
  {:else}
    <Transcriber />
    <PasteTester />
  {/if}
</main>

<style>
  main {
    max-width: 720px;
    margin: 0 auto;
    padding: 2rem 1.5rem 3rem;
  }
  header {
    margin-bottom: 1.5rem;
  }
  h1 {
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0 0 0.25rem;
  }
  .subtitle {
    color: var(--text-dim);
    font-size: 0.875rem;
    margin: 0;
  }
</style>
