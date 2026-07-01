<script>
  import { onMount, onDestroy } from 'svelte';
  import { micLevel } from './stores.js';

  export let active = false;

  let barElement;
  let unsubscribe;

  onMount(() => {
    unsubscribe = micLevel.subscribe((level) => {
      // Optimization: Bypass Svelte's reactivity by grabbing the DOM node
      // and directly mutating its style. This prevents continuous component
      // diffing in parent components when micLevel updates 25x/sec.
      if (barElement) {
        // RMS for typical speech sits around 0.05-0.20, so amplify visually.
        const scale = Math.min(1, level * 3).toFixed(3);
        barElement.style.transform = `scaleX(${scale})`;
      }
    });
  });

  onDestroy(() => {
    if (unsubscribe) unsubscribe();
  });
</script>

<div class="meter" class:active>
  <!-- Optimization: Hardware accelerate high-frequency UI updates by using
       transform: scaleX instead of width. This bypasses the main thread's
       expensive layout and reflow calculations for every incoming audio frame. -->
  <div bind:this={barElement} class="bar"></div>
</div>

<style>
  .meter {
    flex: 1;
    height: 10px;
    background: var(--surface-2);
    border-radius: 5px;
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .bar {
    height: 100%;
    width: 100%;
    background: linear-gradient(90deg, var(--success), #ffeb3b 70%, var(--accent));
    transition: transform 60ms linear;
    transform-origin: left;
    transform: scaleX(0);
  }
  .meter:not(.active) .bar {
    background: var(--text-dim);
    opacity: 0.25;
    transform: scaleX(0) !important;
  }
</style>
