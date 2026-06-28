<script>
  export let level = 0; // 0..1 RMS
  export let active = false;

  // RMS for typical speech sits around 0.05-0.20, so amplify visually.
  $: scale = Math.min(1, level * 3).toFixed(3);
</script>

<div class="meter" class:active>
  <!-- Optimization: Hardware accelerate high-frequency UI updates by using
       transform: scaleX instead of width. This bypasses the main thread's
       expensive layout and reflow calculations for every incoming audio frame. -->
  <div class="bar" style="transform: scaleX({scale})"></div>
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
