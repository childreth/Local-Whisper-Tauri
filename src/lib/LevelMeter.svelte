<script>
  export let level = 0; // 0..1 RMS
  export let active = false;

  // RMS for typical speech sits around 0.05-0.20, so amplify visually.
  $: width = Math.min(100, Math.round(level * 100 * 3));
</script>

<div class="meter" class:active>
  <div class="bar" style="width: {width}%"></div>
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
    background: linear-gradient(90deg, var(--success), #ffeb3b 70%, var(--accent));
    transition: width 60ms linear;
    width: 0%;
  }
  .meter:not(.active) .bar {
    background: var(--text-dim);
    opacity: 0.25;
    width: 0% !important;
  }
</style>
