<script>
  import { onMount } from 'svelte';
  import { micLevel } from './stores.js';

  export let active = false;

  let barElement;

  onMount(() => {
    // Optimization: Subscribe directly to the high-frequency store in the leaf component
    // and imperatively mutate the DOM. This bypasses Svelte's reactivity system completely,
    // preventing the parent Transcriber component from needlessly re-rendering 25 times
    // a second during active recording.
    const unsubscribe = micLevel.subscribe((level) => {
      if (barElement) {
        // RMS for typical speech sits around 0.05-0.20, so amplify visually.
        // Optimization: Rely on native string coercion within the template literal
        // instead of calling explicit string conversion methods like .toFixed() to
        // reduce continuous object allocation and GC pressure in this high-frequency loop.
        // Also replace Math.min with inline ternary to avoid function call overhead.
        const v = level * 3;
        const scale = v > 1 ? 1 : v;
        barElement.style.transform = `scaleX(${scale})`;
      }
    });

    return unsubscribe;
  });
</script>

<div class="meter" class:active>
  <!-- Optimization: Hardware accelerate high-frequency UI updates by using
       transform: scaleX instead of width. This bypasses the main thread's
       expensive layout and reflow calculations for every incoming audio frame. -->
  <div bind:this={barElement} class="bar" style="transform: scaleX(0)"></div>
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
    /* Optimization: Removed transition. Since store subscription manually
       mutates scaleX at ~25fps, CSS transitions just force the browser to
       interpolate and discard animations on every frame, wasting CPU. */
    will-change: transform;
    transform-origin: left;
    transform: scaleX(0);
  }
  .meter:not(.active) .bar {
    background: var(--text-dim);
    opacity: 0.25;
    transform: scaleX(0) !important;
  }
</style>
