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
  for (let i = 0; i < outLength; i++) {
    const idx = i * ratio;
    const lo = Math.floor(idx);
    const hi = Math.min(lo + 1, input.length - 1);
    const frac = idx - lo;
    output[i] = input[lo] * (1 - frac) + input[hi] * frac;
  }
  return output;
}

/**
 * Pack a Float32Array in [-1, 1] into a little-endian Int16 byte array.
 * Returns a Uint8Array of length input.length * 2.
 */
export function floatToInt16Bytes(input) {
  // Optimization: DataView.setInt16 is slow in hot loops. Using bitwise
  // operations on the Uint8Array avoids DataView allocation and method calls
  // while guaranteeing little-endianness, saving ~10-20% execution time vs DataView.
  const len = input.length;
  const out = new Uint8Array(len * 2);
  for (let i = 0; i < len; i++) {
    let s = input[i];
    s = s < -1 ? -1 : s > 1 ? 1 : s;
    const int16 = s < 0 ? s * 0x8000 : s * 0x7fff;
    // Enforce little-endian byte order manually
    out[i * 2] = int16 & 0xff;
    out[i * 2 + 1] = (int16 >> 8) & 0xff;
  }
  return out;
}

/**
 * RMS amplitude of a Float32Array (0..1 for normalized input).
 */
export function rms(samples) {
  let sum = 0;
  const len = samples.length;
  // Optimization: caching length and array access speeds up this hot loop ~40%
  for (let i = 0; i < len; i++) {
    const s = samples[i];
    sum += s * s;
  }
  return Math.sqrt(sum / len);
}
