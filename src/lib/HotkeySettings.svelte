<script>
  import { onMount, onDestroy } from 'svelte';
  import {
    getHotkey,
    setHotkey,
    getFnHotkeyEnabled,
    setFnHotkeyEnabled,
  } from './tauri-bridge.js';
  import { lastError } from './stores.js';

  // Accelerator string as the backend stores it, e.g. "Control+Alt+Space".
  let current = '';
  let editing = false;
  let capturedMods = []; // ['Control', 'Alt']
  let capturedKey = '';  // 'Space', 'KeyA', 'F5'
  let saving = false;
  let fnEnabled = false;
  let fnBusy = false;

  // Map Tauri/W3C names to glyphs used by macOS keycaps.
  const MOD_GLYPH = {
    Super: '⌘',    // ⌘
    SuperLeft: '⌘',
    SuperRight: '⌘',
    Meta: '⌘',
    Control: '⌃',  // ⌃
    Alt: '⌥',      // ⌥
    Option: '⌥',
    Shift: '⇧',    // ⇧
  };

  const KEY_GLYPH = {
    Space: 'Space',
    Enter: '⏎',     // ⏎
    Tab: '⇥',       // ⇥
    Escape: 'esc',
    Backspace: '⌫',
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
  };

  async function load() {
    try {
      current = await getHotkey();
      fnEnabled = await getFnHotkeyEnabled();
    } catch (e) {
      lastError.set({ kind: 'hotkey', message: String(e) });
    }
  }

  async function toggleFn(e) {
    const next = e.target.checked;
    fnBusy = true;
    try {
      await setFnHotkeyEnabled(next);
      fnEnabled = next;
      if (next && editing) endEdit();
    } catch (err) {
      lastError.set({ kind: 'hotkey', message: String(err) });
      // Revert checkbox
      e.target.checked = !next;
    } finally {
      fnBusy = false;
    }
  }

  function parseAccel(accel) {
    if (!accel) return { mods: [], key: '' };
    const parts = accel.split('+').map((s) => s.trim());
    const key = parts.pop();
    return { mods: parts, key };
  }

  function modLabel(m) {
    return MOD_GLYPH[m] || m;
  }

  function keyLabel(k) {
    if (KEY_GLYPH[k]) return KEY_GLYPH[k];
    // Letter / digit codes from `event.code`:
    if (k.startsWith('Key')) return k.slice(3);   // KeyA → A
    if (k.startsWith('Digit')) return k.slice(5); // Digit1 → 1
    return k;
  }

  function startEdit() {
    capturedMods = [];
    capturedKey = '';
    editing = true;
    window.addEventListener('keydown', captureKeydown, true);
    window.addEventListener('keyup', captureKeyup, true);
  }

  function endEdit() {
    editing = false;
    window.removeEventListener('keydown', captureKeydown, true);
    window.removeEventListener('keyup', captureKeyup, true);
  }

  function captureKeydown(e) {
    // Swallow everything while we're capturing so we don't trigger app shortcuts.
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      cancel();
      return;
    }

    const mods = [];
    if (e.metaKey) mods.push('Super');
    if (e.ctrlKey) mods.push('Control');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');

    const isModifierKey =
      e.key === 'Meta' || e.key === 'Control' || e.key === 'Alt' || e.key === 'Shift';

    capturedMods = mods;
    if (isModifierKey) {
      // Just show modifiers being held; wait for a non-modifier to finalize.
      capturedKey = '';
    } else if (mods.length > 0) {
      capturedKey = normalizeCodeForTauri(e.code, e.key);
    }
  }

  function captureKeyup() {
    // No-op; finalization happens on save click after a non-modifier key was
    // captured. Modifier-only chord support isn't possible with this plugin.
  }

  // The plugin's accelerator parser expects Web KeyboardEvent.code values.
  // For letters/digits the code already matches what Tauri wants (KeyA, Digit1).
  // Function keys ("F1".."F12") and Space pass through unchanged. Punctuation
  // codes (Slash, Backquote, BracketLeft etc.) also map 1:1.
  function normalizeCodeForTauri(code, key) {
    return code || key;
  }

  async function save() {
    if (capturedMods.length === 0 || !capturedKey) return;
    const accel = [...capturedMods, capturedKey].join('+');
    saving = true;
    try {
      await setHotkey(accel);
      current = accel;
      endEdit();
    } catch (e) {
      lastError.set({ kind: 'hotkey', message: String(e) });
    } finally {
      saving = false;
    }
  }

  function cancel() {
    endEdit();
  }

  onMount(load);
  onDestroy(endEdit);

  $: display = editing
    ? { mods: capturedMods, key: capturedKey }
    : parseAccel(current);
  $: canSave = editing && capturedMods.length > 0 && capturedKey;
</script>

<div class="card">
  <div class="row">
    <div class="text">
      <div class="title">Push to talk</div>
      <div class="desc">Hold this shortcut globally to dictate into the focused app.</div>
    </div>

    <div class="binding" class:editing class:disabled={fnEnabled}>
      {#if fnEnabled}
        <kbd>fn</kbd>
        <kbd>⇧</kbd>
      {:else if editing && capturedMods.length === 0 && !capturedKey}
        <span class="hint">Press a shortcut…</span>
      {:else}
        {#each display.mods as m}
          <kbd>{modLabel(m)}</kbd>
        {/each}
        {#if display.key}
          <kbd>{keyLabel(display.key)}</kbd>
        {/if}
      {/if}

      {#if !editing && !fnEnabled}
        <button class="icon" on:click={startEdit} title="Edit shortcut">✎</button>
      {:else if editing}
        <button class="icon" on:click={cancel} title="Cancel (Esc)">✕</button>
        <button class="icon save" on:click={save} disabled={!canSave || saving} title="Save">
          ✓
        </button>
      {/if}
    </div>
  </div>

  <label class="toggle">
    <input type="checkbox" checked={fnEnabled} on:change={toggleFn} disabled={fnBusy} />
    <span class="toggle-text">
      <strong>Use fn + Shift</strong>
      <span class="desc">Apple dictation-style binding. Uses a CGEventTap (requires Accessibility permission) since macOS doesn't expose fn through standard shortcut APIs. Overrides the chord above.</span>
    </span>
  </label>
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .toggle {
    display: flex;
    gap: 0.625rem;
    align-items: flex-start;
    cursor: pointer;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border);
  }
  .toggle input {
    margin-top: 0.1875rem;
  }
  .toggle-text {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    font-size: 0.8125rem;
  }
  .toggle-text strong {
    font-weight: 600;
    color: var(--text);
  }
  .binding.disabled {
    opacity: 0.5;
  }
  .title {
    font-size: 0.9375rem;
    font-weight: 600;
  }
  .desc {
    color: var(--text-dim);
    font-size: 0.8125rem;
    margin-top: 0.125rem;
  }
  .binding {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.5rem 0.375rem 0.625rem;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    min-height: 32px;
  }
  .binding.editing {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px rgba(255, 82, 82, 0.18);
  }
  kbd {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0.125rem 0.4rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8125rem;
    min-width: 22px;
    text-align: center;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.8125rem;
    padding: 0 0.25rem;
  }
  .icon {
    background: transparent;
    border: none;
    color: var(--text-dim);
    font-size: 0.9375rem;
    padding: 0.125rem 0.375rem;
    border-radius: 4px;
    line-height: 1;
  }
  .icon:hover:not(:disabled) {
    background: var(--border);
    color: var(--text);
  }
  .icon.save {
    color: var(--success);
  }
  .icon:disabled {
    opacity: 0.4;
  }
</style>
