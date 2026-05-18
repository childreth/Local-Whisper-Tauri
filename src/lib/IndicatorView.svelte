<script>
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  const BAR_COUNT = 9;
  // Center bars react strongest to level; edges stay subtle. Bias values
  // produce the classic "tall in the middle" dictation look.
  const BAR_BIAS = [0.45, 0.6, 0.78, 0.92, 1.0, 0.92, 0.78, 0.6, 0.45];

  let level = 0; // smoothed 0..1
  let transcribing = false;
  let phase = 0;
  let raf = null;
  let unlisten = null;

  function tick() {
    phase += 0.18;
    // Decay level so when audio stops the bars settle.
    level *= 0.85;
    raf = requestAnimationFrame(tick);
  }

  onMount(async () => {
    raf = requestAnimationFrame(tick);
    unlisten = await listen('indicator:level', (e) => {
      const v = Math.min(1, Math.max(0, (e.payload?.level ?? 0) * 3));
      // Take max so spikes are visible immediately, decay smooths the fall.
      if (v > level) level = v;
      transcribing = !!e.payload?.transcribing;
    });
  });

  onDestroy(() => {
    if (raf) cancelAnimationFrame(raf);
    if (unlisten) unlisten();
  });

  // Per-bar height: base level shaped by bias plus a small sinusoidal wobble
  // so the bars look "alive" even at low input.
  function barHeight(i) {
    const wobble = (Math.sin(phase + i * 0.7) + 1) / 2; // 0..1
    const base = level * BAR_BIAS[i];
    const idle = 0.08 + wobble * 0.06;
    const v = Math.max(idle, base + wobble * 0.08 * level);
    return Math.min(1, v);
  }
</script>

<div class="pill" class:transcribing>
  {#each BAR_BIAS as _, i}
    <div class="bar" style="height: {Math.round(barHeight(i) * 100)}%"></div>
  {/each}
</div>

<style>
  :global(html), :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    -webkit-user-select: none;
    user-select: none;
  }
  :global(#app) {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .pill {
    width: 160px;
    height: 36px;
    background: rgba(20, 20, 22, 0.92);
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 0 18px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
  }
  .bar {
    width: 3px;
    min-height: 3px;
    border-radius: 2px;
    background: linear-gradient(180deg, #ffffff, #c7c7cc);
    transition: height 60ms linear;
  }
  .pill.transcribing .bar {
    background: linear-gradient(180deg, #7dd3fc, #38bdf8);
  }
</style>
