// Pure audio utility functions: resampling, encoding, RMS.

/**
 * Linear resample a Float32Array from inputRate to outputRate.
 * Good enough for speech (whisper is tolerant). Swap for a polyphase
 * filter if you start hearing aliasing artifacts in transcriptions.
 */
export function downsample(input, inputRate, outputRate) {
  if (inputRate === outputRate) return input;
  const ratio = inputRate / outputRate;
  const outLength = Math.floor(input.length / ratio);
  const output = new Float32Array(outLength);
  const len = input.length - 1;
  for (let i = 0; i < outLength; i++) {
    const idx = i * ratio;
    const lo = Math.floor(idx);
    const hi = lo < len ? lo + 1 : len;
    const frac = idx - lo;
    output[i] = input[lo] * (1 - frac) + input[hi] * frac;
  }
  return output;
}

const IS_LITTLE_ENDIAN = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;

/**
 * Pack a Float32Array in [-1, 1] into a little-endian Int16 byte array.
 * Returns a Uint8Array of length input.length * 2.
 */
export function floatToInt16Bytes(input) {
  if (IS_LITTLE_ENDIAN) {
    // Fast path: map directly into Int16Array, returning its underlying buffer.
    const out = new Int16Array(input.length);
    for (let i = 0; i < input.length; i++) {
      let s = Math.max(-1, Math.min(1, input[i]));
      out[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
    }
    return new Uint8Array(out.buffer);
  }

  // Fallback for big-endian systems (DataView handles the little-endian write)
  const out = new Uint8Array(input.length * 2);
  const view = new DataView(out.buffer);
  for (let i = 0; i < input.length; i++) {
    let s = Math.max(-1, Math.min(1, input[i]));
    s = s < 0 ? s * 0x8000 : s * 0x7fff;
    view.setInt16(i * 2, s, true);
  }
  return out;
}

/**
 * RMS amplitude of a Float32Array (0..1 for normalized input).
 */
export function rms(samples) {
  let sum = 0;
  const len = samples.length;
  for (let i = 0; i < len; i++) {
    const s = samples[i];
    sum += s * s;
  }
  return Math.sqrt(sum / len);
}
