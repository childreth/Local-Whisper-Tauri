<script>
  import { pasteText } from './tauri-bridge.js';

  const TEST_STRING = 'Hello from Local Whisper 👋';
  const COUNTDOWN_SECONDS = 3;

  let countdown = 0;
  let status = '';
  let timer = null;

  function startCountdown() {
    if (timer) return;
    countdown = COUNTDOWN_SECONDS;
    status = `Switch to your target app. Pasting in ${countdown}…`;

    timer = setInterval(async () => {
      countdown -= 1;
      if (countdown > 0) {
        status = `Switch to your target app. Pasting in ${countdown}…`;
        return;
      }
      clearInterval(timer);
      timer = null;
      try {
        await pasteText(TEST_STRING);
        status = 'Pasted. If nothing appeared, check Accessibility permission.';
      } catch (e) {
        status = `Paste failed: ${e}`;
      }
    }, 1000);
  }
</script>

<div class="tester">
  <button on:click={startCountdown} disabled={timer !== null}>
    Test paste (3s countdown)
  </button>
  {#if status}
    <p class="status">{status}</p>
  {/if}
</div>

<style>
  .tester {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem 1.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  button {
    align-self: flex-start;
    padding: 0.5rem 0.875rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .status {
    margin: 0;
    color: var(--text-dim);
    font-size: 0.8125rem;
  }
</style>
