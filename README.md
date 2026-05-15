# Local Whisper 🎙️

A desktop speech-to-text app that runs entirely on your own machine. No cloud, no accounts, no audio uploads. You press record, you talk, you get text — and the audio never leaves your computer.

Built with Tauri v2 (Rust + native webview), Svelte (vanilla JavaScript), and [whisper.cpp](https://github.com/ggerganov/whisper.cpp) via the [`whisper-rs`](https://github.com/tazz4843/whisper-rs) Rust bindings.

## Why local STT? 🤔

Most consumer transcription tools — Otter, Rev, Google Live Transcribe, Apple's dictation — stream your audio to a remote server. That's fine for casual use, but it's a non-starter when:

- You're transcribing a private conversation, an interview under NDA, a therapy session, a legal meeting, or anything you'd be uncomfortable putting through someone else's data pipeline.
- You're somewhere with no internet and you still want dictation to work.
- You don't want to be billed per-minute, juggle API keys, or hit rate limits.
- You're building on top of transcription and want predictable latency without round-tripping to a server.

Local Whisper does the entire job on-device. Audio is captured by the browser engine, encoded to PCM, handed to a Rust process, and run through a Whisper model loaded into local memory. No network call ever touches the audio.

## What you get 📦

- **Push-to-record** with a Space-bar shortcut.
- **Automatic chunking** — speak in natural sentences, pause for ~0.8 seconds, and that utterance gets transcribed.
- **Live mic level meter** so you can tell the app is hearing you.
- **Copy-all transcript** to clipboard, or clear and start fresh.
- **First-run model download** with a progress bar (~150 MB, one-time).
- **Cold cached after first launch** — the model is loaded into memory at startup, so transcription is fast on every invocation.

## How it works 🔧

There are two halves to the app: a Svelte frontend that handles the UI and microphone capture, and a Rust backend that runs the actual speech model. They talk to each other over Tauri's IPC bridge.

**1. Mic capture (frontend).** When you hit Record, the app asks the browser for microphone access via `getUserMedia` and pipes the audio into a Web Audio `AudioContext`. An `AudioWorkletNode` runs in a separate audio thread and emits raw 32-bit float PCM frames to the main thread.

**2. Voice activity detection (frontend).** Each incoming frame is measured for RMS amplitude. When the RMS stays below a threshold for ~800 ms, the accumulated audio is considered an "utterance" and flushed downstream. This is a deliberately simple silence-based VAD — it's not as sophisticated as a neural VAD like Silero, but it's good enough for dictation-style use and costs zero CPU.

**3. Format conversion (frontend).** The browser captures at its native rate (usually 48 kHz). Whisper wants 16 kHz mono int16, so the frontend downsamples and packs the floats into a little-endian Int16 byte stream before sending it across the IPC bridge.

**4. Inference (backend).** On startup, Rust loads the Whisper model file into a `WhisperContext` and holds it in shared state for the lifetime of the app. When a transcribe request comes in, the Int16 bytes are decoded back to float samples, then handed to `whisper-rs` on a blocking thread (`tokio::task::spawn_blocking`) so the async runtime doesn't stall. The output text is concatenated from all returned segments and sent back to the UI.

**5. Display (frontend).** While inference runs, the UI shows a "…" placeholder for the pending segment. When the result returns, the placeholder is replaced with the transcribed text. Multiple utterances can be in flight at once — each is matched by id, so order is preserved.

```
[Svelte UI]
  ↓ getUserMedia → AudioContext → AudioWorklet
  ↓ raw Float32 PCM frames (native rate, usually 48 kHz)
  ↓ amplitude VAD: flush after ~800 ms below threshold
  ↓ downsample to 16 kHz, pack Float32 → Int16
  ↓ invoke('transcribe', { pcm })
[Tauri Rust backend]
  ↓ WhisperContext held in tauri::State (loaded once at startup)
  ↓ spawn_blocking → whisper_rs inference
  ↓ collect segments → concatenated text
[Svelte UI] ← transcript segment appended, "…" placeholder replaced
```

## Design decisions 💡

A few choices that shape the app significantly:

**`whisper-rs` in-process, not a sidecar binary.** The most common pattern in Tauri whisper tutorials is to ship the `whisper.cpp` CLI as a sidecar and invoke it once per audio chunk. That works, but it's slow: each invocation pays a ~100ms process startup, plus ~1-2 seconds to load the 150 MB model from disk, plus temp WAV file I/O. By embedding `whisper-rs` directly in the Rust process, the model stays in memory across calls. Per-utterance latency drops from 2-4 seconds to ~200-500 ms.

**AudioWorklet, not MediaRecorder.** `MediaRecorder` is the obvious choice for capturing audio in the browser, but it has a sharp edge: when you call `.start(timeslice)`, only the *first* chunk contains a valid webm/opus container header. Subsequent chunks aren't independently decodable. AudioWorklet sidesteps this entirely by giving you raw PCM samples directly — no decoding step needed.

**Silence-based chunking, not true streaming.** Whisper.cpp has a streaming mode, but it adds complexity (overlapping inference windows, partial-result management, context handoff between windows). For dictation use, silence-based chunking is simpler and feels natural — speak a thought, pause, see it appear.

**Svelte over React/Vue.** Smaller bundle, less ceremony, the reactive store model maps cleanly to the audio pipeline. Vanilla JavaScript (no TypeScript) was a personal preference — easy to lift into.

**Tauri v2 over Electron.** Tauri ships native webview instead of bundling Chromium, so the resulting app is ~10 MB instead of ~150 MB before the model. The Rust backend is also a much better home for whisper-rs FFI than a Node child process.

**English-only `ggml-small.en`.** The English-specific Whisper models are notably better than the multilingual ones at the same size. See the dedicated section below for the trade-offs across model sizes.

## Model choice & performance 📊

Whisper comes in five sizes — `tiny`, `base`, `small`, `medium`, `large` — and each has an English-specific variant (`.en`) that's notably better than the multilingual version at the same parameter count. The trade-off is straightforward: bigger model → better accuracy → more memory, more compute, more disk.

Here's the practical cost on a modern Mac laptop:

| Model | File size | RAM (in-process) | Per-utterance latency | Quality |
|---|---|---|---|---|
| `tiny.en` | 39 MB | ~150 MB | ~100 ms | Rough — fine for keyword spotting, weak for sentences |
| `base.en` | 74 MB | ~250 MB | ~200 ms | Decent — visibly worse on names, numbers, technical terms |
| **`small.en`** ← we use this | **148 MB** | **~520 MB total** | **~300-500 ms** | **Strong — handles dictation, accents, technical vocab well** |
| `medium.en` | 469 MB | ~1.2 GB | ~1-2 s | Excellent — diminishing returns for dictation |
| `large-v3` (multilingual only) | 1.5 GB | ~3 GB | ~3-5 s | Best — overkill for English-only on-device use |

`small.en` is the sweet spot for desktop dictation on a 16 GB laptop. `base.en` is tempting on memory grounds but the accuracy drop is real — you'll notice it on names, numbers, and technical jargon. `medium.en` is better but 3× the memory and 3× the latency for marginal gains on conversational speech.

### Real-world memory footprint

Measured in macOS Activity Monitor with `small.en` loaded and actively transcribing: **~520 MB total** for the entire app (model + native webview + Rust runtime + everything).

For comparison, the same model running through `transformers.js` in a browser-based app would typically land at **700 MB - 1 GB**, for two reasons:

1. **Model lives in native memory, not the JS heap.** V8 wraps every typed array with header overhead and can't pack things as densely as raw `malloc`. For a 150 MB model, that easily costs 100-200 MB extra in JS-land.
2. **No JS-side ML runtime.** ONNX Runtime Web is ~30 MB of WASM plus runtime structures. `whisper.cpp` is just a C++ blob the Rust process loads directly — no separate runtime, no WASM JIT, no GC pressure.

This is the main reason the in-process `whisper-rs` approach was chosen over a browser-based ML runtime: roughly half the memory at the same accuracy.

### Going smaller

If 520 MB is too much for your use case, two ways to push it down without changing accuracy floor much:

- **Drop to `base.en`** — ~80 MB on disk, ~250 MB total runtime. Visibly worse quality, but a real memory win.
- **Use a quantized variant** like `ggml-small.en-q5_0.bin` — same architecture, lower-precision weights. ~88 MB on disk, ~200 MB loaded. Small quality hit, big memory savings. Quantized models are published at the same Hugging Face repo (`ggerganov/whisper.cpp`).

To swap models, change `MODEL_FILE` and `MODEL_URL` in `src-tauri/src/model.rs`. The download flow handles the rest.

## Privacy & data handling 🔒

This section matters because the headline feature is "nothing leaves your machine," and it's worth being precise about that.

- **Audio** is captured by the browser, processed in memory, sent over Tauri's local IPC channel to the Rust process, and then handed to whisper-rs which runs C++ inference in the same process. It's never written to disk and never sent over the network.
- **Transcribed text** lives in the Svelte component state. It's not persisted anywhere unless you explicitly copy it to your clipboard.
- **The model file** (`ggml-small.en.bin`) is downloaded once on first launch from Hugging Face over HTTPS, stored in the OS application data directory, and never re-fetched unless you delete it.
- **No telemetry, no analytics, no crash reporting.** The app makes exactly one outbound network call in its lifetime: the model download.

If you fork this app and want to confirm: search the entire Rust source for `reqwest::` and `http`, and the JavaScript source for `fetch(`, `XMLHttpRequest`, and `WebSocket`. The only network call you'll find is in `src-tauri/src/model.rs`, calling Hugging Face.

## Setup ⚙️

```bash
# Prerequisites
brew install cmake          # whisper-rs compiles whisper.cpp from source
# Rust 1.77+ already installed via rustup or Homebrew
# Node 18+

# Install + run
npm install
npm run tauri dev
```

First launch takes 1-3 minutes to compile whisper-rs (subsequent builds are incremental and fast). The first time you hit Record, macOS will prompt for microphone access — say yes, or fix it later in System Settings → Privacy & Security → Microphone.

On first transcription request, the app downloads `ggml-small.en.bin` (~150 MB) from Hugging Face. Subsequent launches load the cached model from disk.

## Project layout 🗂️

```
Local Whisper (Tauri)/
├── package.json, vite.config.js, index.html
├── public/audio-worklet.js      ← runs in AudioWorklet scope
├── src/
│   ├── main.js, app.css, App.svelte
│   └── lib/
│       ├── Transcriber.svelte   ← mic capture + VAD
│       ├── RecordButton, LevelMeter, TranscriptView, ModelLoader, ErrorBanner
│       ├── stores.js            ← appState, transcript, micLevel, modelProgress, lastError
│       ├── audio-utils.js       ← downsample + Int16 packing + RMS
│       └── tauri-bridge.js      ← invoke wrappers
└── src-tauri/
    ├── Cargo.toml, tauri.conf.json, Info.plist, build.rs
    ├── capabilities/default.json
    └── src/
        ├── main.rs, lib.rs       ← command registration, app state
        ├── whisper.rs            ← WhisperEngine + run_inference
        ├── model.rs              ← download + SHA-256 + progress emit
        └── error.rs              ← TranscribeError enum
```

## Configuration reference 🛠️

All tunables live in source — Vite hot-reloads the frontend; `cargo` rebuilds the backend on save.

### VAD + utterance chunking (`src/lib/Transcriber.svelte`)

| Param | Default | What it does |
|---|---|---|
| `TARGET_SAMPLE_RATE` | `16000` | Sample rate sent to whisper. Don't change — whisper requires 16kHz. |
| `SILENCE_THRESHOLD` | `0.02` | RMS amplitude below this counts as silence. Raise to `0.04–0.06` if your noise floor triggers false detections. |
| `SILENCE_DURATION_MS` | `800` | Flush an utterance after this much continuous silence. Lower = snappier feedback, higher = better sentence boundaries. |
| `MIN_UTTERANCE_MS` | `300` | Ignore utterances shorter than this. Raise to `500–700` to drop more false-positive blips (coughs, clicks). |
| `MAX_UTTERANCE_MS` | `15000` | Force-flush after this duration even without silence — prevents one runaway monologue from never showing up. |

### Microphone capture (`getUserMedia` in `Transcriber.svelte`)

| Param | Default | What it does |
|---|---|---|
| `channelCount` | `1` | Mono input. Whisper is mono-only; don't change. |
| `echoCancellation` | `true` | Browser-level AEC. Disable if you're feeding system audio through a virtual cable. |
| `noiseSuppression` | `true` | Browser-level NS. Helpful for fan / HVAC noise. |
| `autoGainControl` | `true` | Auto-leveling. Disable if you want raw mic levels for the meter. |

### Whisper inference (`src-tauri/src/whisper.rs`)

| Param | Default | What it does |
|---|---|---|
| `SamplingStrategy` | `Greedy { best_of: 1 }` | Fast, deterministic decoding. Swap to `BeamSearch { beam_size: 5, patience: 1.0 }` for slightly better quality at ~3x cost. |
| `language` | `Some("en")` | Set to `None` for auto-detect (only with multilingual models). |
| `translate` | `false` | When true, translates source language → English. |
| `print_progress` / `_special` / `_realtime` / `_timestamps` | `false` | All off — we just want clean text out. |
| `n_threads` | `available_parallelism` (≈ CPU count) | Falls back to 4 if detection fails. |
| Min sample length | `1600` (≈100ms) | Audio shorter than this is rejected before inference. |

**Hallucination-reduction flags worth adding** (not currently set):

| Flag | Suggested | What it does |
|---|---|---|
| `set_no_context(true)` | recommended | Disables carrying decoder state between calls. Reduces "made up" text on isolated short clips. |
| `set_suppress_blank(true)` | recommended | Suppresses hallucinated tokens at the start of segments. |
| `set_no_speech_thold(0.6)` | optional | Stronger threshold for "this segment is silence" — drops more dead-air segments. |

### Model download (`src-tauri/src/model.rs`)

| Param | Default | What it does |
|---|---|---|
| `MODEL_URL` | `ggerganov/whisper.cpp/ggml-small.en.bin` on Hugging Face | The single model URL fetched on first run. |
| `MODEL_FILE` | `"ggml-small.en.bin"` | Filename in the app data dir. Change to `medium.en.bin` / `base.en.bin` etc. if you swap models. |
| `EXPECTED_SHA256` | `None` | Set to a hex string to enforce integrity. Compute once with `shasum -a 256 ggml-small.en.bin`. |
| `PROGRESS_INTERVAL` | `120 ms` | Throttle for `model:progress` events during download. |
| Min file-size sanity check | `10_000_000` bytes | Files smaller than this are treated as corrupt and re-downloaded. |

### Window + bundle (`src-tauri/tauri.conf.json`)

| Param | Default | What it does |
|---|---|---|
| `productName` | `"Local Whisper"` | Display name. |
| `identifier` | `com.chrishildreth.localwhisper` | Reverse-DNS bundle ID. Change for your own namespace before shipping. |
| `windows[0]` size | `820 × 720` (min `480 × 480`) | Resizable. |
| `bundle.macOS.minimumSystemVersion` | `"10.15"` | Bump if you use newer macOS APIs. |
| `NSMicrophoneUsageDescription` | in `Info.plist` | The prompt shown to the user when macOS asks for mic permission. |

## Known limitations ⚠️

- **Whisper hallucinations on silence.** Whisper has a well-documented tendency to invent plausible-sounding text when fed near-silence or static. The recommended `set_no_context` / `set_suppress_blank` flags above mitigate but don't eliminate this. Look for invented "thank you for watching" / "subscribe" phrases — those are training-set ghosts.
- **Amplitude-based VAD is naive.** A proper VAD model (Silero, WebRTC VAD) handles breaths, lip smacks, and background music far better than RMS thresholding. Swap-in opportunity.
- **English only.** Using `ggml-small.en`. To go multilingual, swap the model file and remove the `language="en"` constraint in `whisper.rs`.
- **Model is downloaded, not bundled.** Users need internet on first launch. (Trade-off: keeps the installer small and lets you swap models later.)
- **No code signing or notarization.** `tauri build` produces an unsigned `.app` / `.dmg`. For Mac App Store or Gatekeeper-friendly distribution you'd add signing certs to the bundle config.
- **No persistence.** Transcripts live only in component state — refresh the app and they're gone. Add a tauri-plugin-store or a local SQLite file if you want history.

## Roadmap 🛣️

Reasonable next steps in roughly increasing effort:

- Apply the hallucination-reduction whisper flags (`set_no_context`, `set_suppress_blank`)
- Live "interim" transcript display while an utterance is still accumulating
- Configurable model size picker (base.en / small.en / medium.en) with on-demand download
- Word-level timestamps for richer downstream use
- Persist transcripts to local SQLite with search
- Replace amplitude VAD with Silero VAD (ONNX, ~2 MB add)
- Multilingual model support with language auto-detect
- Packaging: signed `.dmg`, notarization, auto-update channel

## Acknowledgments 🙏

This app is mostly glue. The hard parts are done by:

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — Georgi Gerganov's C++ port of OpenAI's Whisper model with SIMD optimizations.
- [whisper-rs](https://github.com/tazz4843/whisper-rs) — Rust FFI bindings around whisper.cpp.
- [Tauri](https://tauri.app/) — the desktop shell framework.
- [Svelte](https://svelte.dev/) — the frontend framework.
- [OpenAI](https://openai.com/) — the original Whisper model and paper.
