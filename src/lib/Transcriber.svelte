<script>
  import { onMount, onDestroy } from 'svelte';
  import { appState, transcript, micLevel, lastError } from './stores.js';
  import { transcribe } from './tauri-bridge.js';
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

  // === Audio state ===
  let audioCtx = null;
  let mediaStream = null;
  let workletNode = null;
  let sourceNode = null;
  let inputSampleRate = 48000;

  // === Per-utterance accumulator ===
  let frames = []; // Float32Array[]
  let frameSamples = 0;
  let silentMs = 0;
  let speechMs = 0;

  let nextSegmentId = 1;

  function frameMs(samples) {
    return (samples / inputSampleRate) * 1000;
  }

  async function startRecording() {
    try {
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

      resetUtterance();
      appState.set('recording');
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
    await teardown();
    micLevel.set(0);
    appState.set('idle');
  }

  async function teardown() {
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
  }

  function resetUtterance() {
    frames = [];
    frameSamples = 0;
    silentMs = 0;
    speechMs = 0;
  }

  function handleFrame(frame) {
    const level = rms(frame);
    micLevel.set(level);

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

    transcribe(pcm)
      .then((text) => {
        const clean = (text || '').trim();
        transcript.update((segs) =>
          segs.map((s) =>
            s.id === id
              ? { ...s, text: clean || '(silence)', pending: false }
              : s
          )
        );
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

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
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
    Press <kbd>Space</kbd> to toggle. Pauses of ~0.8s flush an utterance.
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
