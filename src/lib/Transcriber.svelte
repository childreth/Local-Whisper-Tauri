<script>
  import { onMount, onDestroy } from 'svelte';
  import { appState, transcript, micLevel, lastError } from './stores.js';
  import { transcribe, onHotkey, pasteText, setIndicatorVisible } from './tauri-bridge.js';
  import { emit } from '@tauri-apps/api/event';
  import { downsample, floatToInt16Bytes, rms } from './audio-utils.js';
  import RecordButton from './RecordButton.svelte';
  import LevelMeter from './LevelMeter.svelte';
  import TranscriptView from './TranscriptView.svelte';

  // === Tunables ===
  const TARGET_SAMPLE_RATE = 16000;
  const SILENCE_THRESHOLD = 0.02; // RMS below this = silence
  const SILENCE_DURATION_MS = 800; // flush after this much silence
  const MIN_UTTERANCE_MS = 300; // ignore blips shorter than this
  const MAX_UTTERANCE_MS = 15000; // safety flush
  // How much audio history to keep around for pre-roll. Captures words spoken
  // at the instant the user hits the hotkey, before the worklet has begun
  // forwarding frames for the new "recording" session.
  const PREROLL_MS = 500;

  // === Audio pipeline (warmed once, kept alive for the app's lifetime) ===
  let audioCtx = null;
  let mediaStream = null;
  let workletNode = null;
  let sourceNode = null;
  let inputSampleRate = 48000;
  let pipelineReady = false;
  let pipelineInitPromise = null;

  // === Pre-roll ring buffer (only populated when NOT actively recording) ===
  let preRoll = []; // Float32Array[]
  let preRollSamples = 0;

  // === Per-utterance accumulator (only populated when actively recording) ===
  let frames = []; // Float32Array[]
  let frameSamples = 0;
  let silentMs = 0;
  let speechMs = 0;

  let nextSegmentId = 1;
  // True when the current recording session was started via the global hotkey;
  // those transcripts get auto-pasted into the focused app.
  let pasteOnComplete = false;

  function frameMs(samples) {
    return (samples / inputSampleRate) * 1000;
  }

  // Open the mic, AudioContext, and worklet once. Idempotent — repeat calls
  // resolve immediately if the pipeline is already up.
  async function initPipeline() {
    if (pipelineReady) return;
    if (pipelineInitPromise) return pipelineInitPromise;

    pipelineInitPromise = (async () => {
      mediaStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          channelCount: 1,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      audioCtx = new AudioContext();
      inputSampleRate = audioCtx.sampleRate;

      await audioCtx.audioWorklet.addModule('/audio-worklet.js');
      workletNode = new AudioWorkletNode(audioCtx, 'pcm-capture');
      workletNode.port.onmessage = (e) => handleFrame(e.data);

      sourceNode = audioCtx.createMediaStreamSource(mediaStream);
      sourceNode.connect(workletNode);
      // No connection to destination — we don't want to play back the mic.

      pipelineReady = true;
    })();

    try {
      await pipelineInitPromise;
    } finally {
      pipelineInitPromise = null;
    }
  }

  async function startRecording() {
    try {
      if (!pipelineReady) {
        // First time, or permission was previously denied. Cold-start it now —
        // first word may still be partially lost on this single call.
        await initPipeline();
      }

      // Seed the utterance with the pre-roll so words spoken at the moment
      // the hotkey fired are captured.
      frames = preRoll.slice();
      frameSamples = preRollSamples;
      // Recompute speech/silence accounting from the pre-roll so the silence
      // timer starts in the right state.
      silentMs = 0;
      speechMs = 0;
      for (const f of frames) {
        const ms = frameMs(f.length);
        if (rms(f) < SILENCE_THRESHOLD) {
          silentMs += ms;
        } else {
          silentMs = 0;
          speechMs += ms;
        }
      }
      preRoll = [];
      preRollSamples = 0;

      appState.set('recording');
      if (pasteOnComplete) {
        try { await setIndicatorVisible(true); } catch {}
      }
    } catch (e) {
      const message =
        e.name === 'NotAllowedError'
          ? 'Microphone permission denied. Open System Settings → Privacy & Security → Microphone and enable Local Whisper.'
          : String(e?.message || e);
      lastError.set({ kind: 'mic-permission', message });
      appState.set('error');
      await teardown();
    }
  }

  async function stopRecording() {
    // Flush any in-progress utterance that meets the minimum length.
    if (speechMs >= MIN_UTTERANCE_MS) {
      flushUtterance();
    }
    resetUtterance();
    // Keep the pipeline warm — do NOT teardown. handleFrame will resume
    // filling the pre-roll buffer.
    micLevel.set(0);
    appState.set('idle');
    pasteOnComplete = false;
    try { await setIndicatorVisible(false); } catch {}
  }

  async function teardown() {
    pipelineReady = false;
    if (workletNode) {
      workletNode.port.onmessage = null;
      workletNode.disconnect();
      workletNode = null;
    }
    if (sourceNode) {
      sourceNode.disconnect();
      sourceNode = null;
    }
    if (mediaStream) {
      mediaStream.getTracks().forEach((t) => t.stop());
      mediaStream = null;
    }
    if (audioCtx) {
      try {
        await audioCtx.close();
      } catch {}
      audioCtx = null;
    }
    preRoll = [];
    preRollSamples = 0;
  }

  function resetUtterance() {
    frames = [];
    frameSamples = 0;
    silentMs = 0;
    speechMs = 0;
  }

  let lastLevelEmit = 0;
  function handleFrame(frame) {
    const level = rms(frame);
    const recording = $appState === 'recording';

    // Mic meter only meaningful while we're "live" — keep it at zero in pre-roll
    // mode so the meter doesn't suggest the app is recording when it isn't.
    micLevel.set(recording ? level : 0);

    if (recording && pasteOnComplete) {
      const now = performance.now();
      if (now - lastLevelEmit > 40) {
        lastLevelEmit = now;
        emit('indicator:level', { level, transcribing: false }).catch(() => {});
      }
    }

    if (!recording) {
      // Keep a rolling window of recent audio so a hotkey press can grab the
      // last PREROLL_MS to capture words spoken at the moment of trigger.
      preRoll.push(frame);
      preRollSamples += frame.length;
      while (preRoll.length > 0 && frameMs(preRollSamples - preRoll[0].length) >= PREROLL_MS) {
        preRollSamples -= preRoll[0].length;
        preRoll.shift();
      }
      return;
    }

    frames.push(frame);
    frameSamples += frame.length;

    const ms = frameMs(frame.length);
    if (level < SILENCE_THRESHOLD) {
      silentMs += ms;
    } else {
      silentMs = 0;
      speechMs += ms;
    }

    const totalMs = frameMs(frameSamples);
    const shouldFlush =
      (silentMs >= SILENCE_DURATION_MS && speechMs >= MIN_UTTERANCE_MS) ||
      totalMs >= MAX_UTTERANCE_MS;

    if (shouldFlush) {
      flushUtterance();
    }
  }

  function flushUtterance() {
    // Concatenate frames into a single Float32Array.
    const combined = new Float32Array(frameSamples);
    let offset = 0;
    for (const f of frames) {
      combined.set(f, offset);
      offset += f.length;
    }
    resetUtterance();

    // Drop if overall energy is too low (false-positive trigger).
    if (rms(combined) < SILENCE_THRESHOLD * 0.5) return;

    const resampled = downsample(combined, inputSampleRate, TARGET_SAMPLE_RATE);
    const pcm = floatToInt16Bytes(resampled);

    const id = nextSegmentId++;
    transcript.update((segs) => [...segs, { id, text: '…', pending: true }]);

    const shouldPaste = pasteOnComplete;
    transcribe(pcm)
      .then(async (text) => {
        const clean = (text || '').trim();
        transcript.update((segs) =>
          segs.map((s) =>
            s.id === id
              ? { ...s, text: clean || '(silence)', pending: false }
              : s
          )
        );
        if (shouldPaste && clean) {
          try {
            await pasteText(clean);
          } catch (e) {
            lastError.set({ kind: 'paste', message: String(e) });
          }
        }
      })
      .catch((e) => {
        transcript.update((segs) =>
          segs.map((s) =>
            s.id === id
              ? { ...s, text: '⚠️ transcription failed', pending: false }
              : s
          )
        );
        lastError.set({ kind: 'transcribe', message: String(e) });
      });
  }

  function toggleRecording() {
    if ($appState === 'recording') stopRecording();
    else if ($appState === 'idle' || $appState === 'error') startRecording();
  }

  function handleKeydown(e) {
    if (e.code !== 'Space') return;
    const tag = e.target?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA') return;
    e.preventDefault();
    toggleRecording();
  }

  let unlistenHotkey = null;

  onMount(async () => {
    window.addEventListener('keydown', handleKeydown);
    // Hold-to-record via global hotkey (⌃⌥Space).
    unlistenHotkey = await onHotkey({
      onDown: () => {
        if ($appState === 'idle' || $appState === 'error') {
          pasteOnComplete = true;
          startRecording();
        }
      },
      onUp: () => {
        if ($appState === 'recording') stopRecording();
      },
    });

    // Warm the mic pipeline now so the first hotkey press doesn't pay the
    // CoreAudio cold-start cost (~200-500 ms). The macOS mic indicator will
    // stay on for the app's lifetime — that's the trade-off for instant
    // capture. If permission isn't granted yet, swallow the error and we'll
    // fall back to lazy init on the first recording attempt.
    try {
      await initPipeline();
    } catch (e) {
      // Common case: user hasn't approved mic access yet. Pipeline stays cold
      // and startRecording() will retry (triggering the system prompt).
      console.warn('mic warm-up skipped:', e?.message || e);
    }
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    if (unlistenHotkey) unlistenHotkey();
    teardown();
  });
</script>

<div class="container">
  <div class="controls">
    <RecordButton
      recording={$appState === 'recording'}
      on:click={toggleRecording}
    />
    <LevelMeter level={$micLevel} active={$appState === 'recording'} />
  </div>

  <p class="hint">
    Press <kbd>Space</kbd> to toggle, or hold <kbd>⌃⌥Space</kbd> globally to record.
  </p>

  <TranscriptView />
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 1.25rem 1.5rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .hint {
    text-align: center;
    color: var(--text-dim);
    font-size: 0.8125rem;
    margin: 0;
  }
  kbd {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.125rem 0.375rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.75rem;
  }
</style>
