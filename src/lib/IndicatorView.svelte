<script>
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  // Array of colors matching the peach/blue/purple aesthetic from the screenshot
  const FIXED_COLORS = [
    '#93c5fd', // blue
    '#d8b4fe', // purple
    '#fdba74', // peach
    '#93c5fd', // blue
    '#93c5fd', // blue
    '#fdba74', // peach
    '#d8b4fe', // purple
    '#93c5fd', // blue
    '#d8b4fe', // purple
    '#fdba74', // peach
    '#93c5fd', // blue
    '#d8b4fe', // purple
    '#93c5fd', // blue
    '#fdba74', // peach
    '#d8b4fe'  // purple
  ];
  
  const BAR_COUNT = FIXED_COLORS.length;
  // Chaotic but natural looking distribution for biases, not just a bell curve
  const BAR_BIAS = Array.from({length: BAR_COUNT}, () => 0.5 + Math.random() * 0.5);

  let level = 0; // smoothed 0..1
  let transcribing = false;
  let phase = 0;
  let raf = null;
  let unlisten = null;
  
  let active = false;
  let activeTimeout = null;
  let activationPending = false;

  // Keep an array of references to the DOM nodes
  let barElements = [];

  function tick() {
    raf = requestAnimationFrame(tick);
    // Optimization: skip Svelte reactive assignments when hidden/idle
    // to prevent background CPU thrashing and continuous DOM recalculations
    if (!active && level < 0.001) return;
    phase += 0.15;
    level *= 0.85; // decay the audio level smoothly

    // Direct DOM mutation for hot loop to bypass Svelte's reactivity system
    // and avoid a full component render/diff on every frame (~60-120fps)
    for (let i = 0; i < BAR_COUNT; i++) {
      if (barElements[i]) {
        // Optimization: Use GPU-accelerated transform (scaleY) instead of height
        // to prevent main thread layout/reflow thrashing on every frame.
        // Base height is 100% (40px). Minimum scale of 0.1 gives 4px minimum height.
        const scale = Math.max(0.1, barHeight(i, level, phase));
        barElements[i].style.transform = `scaleY(${scale})`;
      }
    }
  }

  onMount(async () => {
    raf = requestAnimationFrame(tick);
    unlisten = await listen('indicator:level', (e) => {
      // amplify input slightly for more dramatic effect
      const v = Math.min(1, Math.max(0, (e.payload?.level ?? 0) * 5.0));
      if (v > level) level = v;
      transcribing = !!e.payload?.transcribing;

      if (!active && !activationPending) {
        activationPending = true;
        // Delay adding the active class so the native window has time to appear 
        // on screen BEFORE the CSS transition starts. Otherwise the browser
        // skips the animation because the DOM updates while still invisible.
        setTimeout(() => {
          active = true;
          activationPending = false;
        }, 50);
      }

      clearTimeout(activeTimeout);
      activeTimeout = setTimeout(() => {
        active = false;
        activationPending = false;
      }, 150);
    });
  });

  onDestroy(() => {
    if (raf) cancelAnimationFrame(raf);
    if (unlisten) unlisten();
    clearTimeout(activeTimeout);
  });

  function barHeight(i, currentLevel, currentPhase) {
    const wobble = (Math.sin(currentPhase + i * 0.9) + 1) / 2;
    const base = currentLevel * BAR_BIAS[i];
    const idle = 0.08 + wobble * 0.08;
    const v = Math.max(idle, base + wobble * 0.15 * currentLevel);
    return Math.min(1, Math.max(0.08, v));
  }
</script>

<div class="pill" class:transcribing class:active>
  {#each FIXED_COLORS as color, i}
    <div 
      bind:this={barElements[i]}
      class="bar" 
      style="
          transform: scaleY(0.1);
        background-color: {color};
        box-shadow: 0 0 8px {color};
      "
    ></div>
  {/each}
</div>

<style>
  :global(body.indicator) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    -webkit-user-select: none;
    user-select: none;
  }
  :global(body.indicator #app) {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .pill {
    width: 120px;
    height: 40px;
    background: rgba(20, 20, 22, 0.92);
    border-radius: 999px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 0 20px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    
    /* Inactive state (hidden/recording stopped) */
    opacity: 0;
    transform: scale(0.4);
    filter: blur(5px);
    transition: opacity 0.15s ease-out, transform 0.5s cubic-bezier(.39,0,.17,1), filter 0.5s ease-out;
  }

  .pill.active {
    /* Active state (recording) */
    opacity: 1;
    transform: scale(1);
    filter: blur(0px);
  }

  .bar {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    /* Optimization: Removed transition. Since requestAnimationFrame manually
       mutates height at ~60fps, CSS transitions just force the browser to
       interpolate and discard animations on every frame, wasting CPU. */
    opacity: 0.95;
    transform-origin: center;
    will-change: transform;
  }
</style>
