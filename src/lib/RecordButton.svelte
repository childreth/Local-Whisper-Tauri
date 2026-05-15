<script>
  import { createEventDispatcher } from 'svelte';
  export let recording = false;
  const dispatch = createEventDispatcher();
</script>

<button class:recording on:click={() => dispatch('click')}>
  <span class="dot"></span>
  {recording ? 'Stop' : 'Record'}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: var(--radius);
    font-size: 0.9375rem;
    font-weight: 600;
    transition: background 150ms ease, transform 80ms ease;
    min-width: 110px;
    justify-content: center;
  }
  button:hover {
    background: var(--accent-hover);
  }
  button:active {
    transform: scale(0.97);
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: white;
  }
  button.recording .dot {
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.4; transform: scale(0.7); }
  }
</style>
