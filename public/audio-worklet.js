// AudioWorklet processor: emits raw Float32 mono PCM frames to the main thread.
// Loaded via audioContext.audioWorklet.addModule('/audio-worklet.js').

class PcmCaptureProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    // A standard AudioWorklet processes in blocks of 128 samples.
    // Emitting a message to the main thread on every block causes ~375Hz of main-thread
    // wakeups and GC allocations (at 48kHz). We batch into larger chunks (e.g. 2048) to reduce
    // CPU overhead while keeping latency low enough for VAD (~42ms at 48kHz).
    this.batchSize = 2048;
    this.buffer = new Float32Array(this.batchSize);
    this.offset = 0;
  }

  process(inputs) {
    const input = inputs[0];
    if (!input || input.length === 0) return true;
    const channel = input[0];
    if (!channel || channel.length === 0) return true;

    let inOffset = 0;
    const channelLength = channel.length;
    while (inOffset < channelLength) {
      const remaining = this.batchSize - this.offset;
      const available = channelLength - inOffset;

      // Optimization: Replace Math.min with inline conditional to avoid function call overhead
      const copyCount = available < remaining ? available : remaining;

      // Optimization: Replace channel.subarray().set() with a manual loop to prevent
      // temporary TypedArray allocations and garbage collection pauses in the hot loop.
      for (let i = 0; i < copyCount; i++) {
        this.buffer[this.offset + i] = channel[inOffset + i];
      }

      this.offset += copyCount;
      inOffset += copyCount;

      if (this.offset >= this.batchSize) {
        // Transfer a copy to the main thread for zero-copy delivery.
        // We must copy because this.buffer is reused across iterations.
        const copy = new Float32Array(this.buffer);
        this.port.postMessage(copy, [copy.buffer]);
        this.offset = 0;
      }
    }

    return true;
  }
}

registerProcessor('pcm-capture', PcmCaptureProcessor);
