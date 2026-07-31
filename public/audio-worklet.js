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

      // Optimization: Cache class properties (this.buffer, this.offset) in local variables
      // to prevent redundant property lookup overhead in the real-time critical hot loop.
      const buf = this.buffer;
      const currentOffset = this.offset;

      // Optimization: Replace channel.subarray().set() with a manual loop to prevent
      // temporary TypedArray allocations and garbage collection pauses in the hot loop.
      for (let i = 0; i < copyCount; i++) {
        buf[currentOffset + i] = channel[inOffset + i];
      }

      this.offset += copyCount;
      inOffset += copyCount;

      if (this.offset >= this.batchSize) {
        // Optimization: Transfer the buffer ownership directly to the main thread
        // for true zero-copy delivery, bypassing the O(N) element-wise copy cost of
        // `new Float32Array(this.buffer)` on the real-time audio thread.
        // We then allocate a new buffer to receive the next batch.
        const transferBuffer = this.buffer;
        this.port.postMessage(transferBuffer, [transferBuffer.buffer]);
        this.buffer = new Float32Array(this.batchSize);
        this.offset = 0;
      }
    }

    return true;
  }
}

registerProcessor('pcm-capture', PcmCaptureProcessor);
