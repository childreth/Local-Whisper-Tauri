// AudioWorklet processor: emits raw Float32 mono PCM frames to the main thread.
// Loaded via audioContext.audioWorklet.addModule('/audio-worklet.js').

class PcmCaptureProcessor extends AudioWorkletProcessor {
  process(inputs) {
    const input = inputs[0];
    if (!input || input.length === 0) return true;
    const channel = input[0];
    if (!channel || channel.length === 0) return true;

    // Copy because the underlying buffer is reused each callback,
    // and transfer the copy's buffer for zero-copy delivery.
    const copy = new Float32Array(channel.length);
    copy.set(channel);
    this.port.postMessage(copy, [copy.buffer]);
    return true;
  }
}

registerProcessor('pcm-capture', PcmCaptureProcessor);
