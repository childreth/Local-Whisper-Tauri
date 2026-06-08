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
  const lenMinusOne = input.length - 1;
  for (let i = 0; i < outLength; i++) {
    const idx = i * ratio;
    const lo = Math.floor(idx);
    const hi = lo < lenMinusOne ? lo + 1 : lenMinusOne;
    const frac = idx - lo;
    output[i] = input[lo] + (input[hi] - input[lo]) * frac;
  }
  return output;
}

// Detect if the environment is little-endian (most modern architectures are).
// This allows us to write directly to an Int16Array, bypassing the slower DataView.
const isLittleEndian = new Uint8Array(new Uint16Array([0x00FF]).buffer)[0] === 0xFF;

/**
 * Pack a Float32Array in [-1, 1] into a little-endian Int16 byte array.
 * Returns a Uint8Array of length input.length * 2.
 */
export function floatToInt16Bytes(input) {
  const len = input.length;

  if (isLittleEndian) {
    // Fast path: direct writes to a typed array mapped over the output buffer.
    const int16Buffer = new Int16Array(len);
    for (let i = 0; i < len; i++) {
      let s = input[i];
      s = s < -1 ? -1 : (s > 1 ? 1 : s);
      int16Buffer[i] = s < 0 ? s * 0x8000 : s * 0x7fff;
    }
    return new Uint8Array(int16Buffer.buffer);
  }

  // Slow path: fallback for big-endian systems to ensure correct byte order.
  const out = new Uint8Array(len * 2);
  const view = new DataView(out.buffer);
  for (let i = 0; i < len; i++) {
    let s = input[i];
    s = s < -1 ? -1 : (s > 1 ? 1 : s);
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
    const val = samples[i];
    sum += val * val;
  }
  return Math.sqrt(sum / len);
}
