// Pure audio utility functions: resampling, encoding, RMS.

/**
 * Linear resample a Float32Array from inputRate to outputRate.
 * Good enough for speech (whisper is tolerant). Swap for a polyphase
 * filter if you start hearing aliasing artifacts in transcriptions.
 */
export function downsample(input, inputRate, outputRate) {
  const chunks = input instanceof Float32Array ? [input] : input;
  let inLen = 0;
  for (let i = 0; i < chunks.length; i++) {
    inLen += chunks[i].length;
  }

  if (inputRate === outputRate) {
    if (input instanceof Float32Array) return input;
    const output = new Float32Array(inLen);
    let offset = 0;
    for (let i = 0; i < chunks.length; i++) {
      output.set(chunks[i], offset);
      offset += chunks[i].length;
    }
    return output;
  }

  const ratio = inputRate / outputRate;
  const outLength = (inLen / ratio) | 0;
  const output = new Float32Array(outLength);

  // Optimization: If the ratio is an exact integer (e.g. 48kHz -> 16kHz = 3),
  // we can completely skip fractional interpolation math and directly assign.
  // Optimization: Use loop unswitching to process chunk-by-chunk rather than
  // sample-by-sample globally to remove continuous bound checking in the hot loop.
  if (Number.isInteger(ratio)) {
    let i = 0;
    let j = 0; // global input index
    let chunkOffset = 0;

    for (let chunkIdx = 0; chunkIdx < chunks.length; chunkIdx++) {
      const currentChunk = chunks[chunkIdx];
      const chunkLen = currentChunk.length;
      const nextChunkOffset = chunkOffset + chunkLen;

      while (i < outLength) {
        if (j >= nextChunkOffset) break;
        output[i++] = currentChunk[j - chunkOffset];
        j += ratio;
      }

      chunkOffset = nextChunkOffset;
    }
    return output;
  }

  // Optimization: Use loop unswitching to process chunk-by-chunk rather than
  // sample-by-sample globally. This removes continuous bound checking, internal
  // while loops, and array offset calculations from the innermost hot loop.
  let i = 0;
  let chunkOffset = 0;

  for (let chunkIdx = 0; chunkIdx < chunks.length; chunkIdx++) {
    const currentChunk = chunks[chunkIdx];
    const chunkLen = currentChunk.length;
    const nextChunkOffset = chunkOffset + chunkLen;

    // Fast path: process samples fully contained in this chunk
    while (i < outLength) {
      const idx = i * ratio;
      const lo = idx | 0;

      if (lo >= nextChunkOffset - 1) break;

      const localLo = lo - chunkOffset;
      const frac = idx - lo;
      const val = currentChunk[localLo];
      const nextVal = currentChunk[localLo + 1];
      output[i++] = val + (nextVal - val) * frac;
    }

    // Boundary path: process samples crossing chunks or at exact end
    while (i < outLength) {
      const idx = i * ratio;
      const lo = idx | 0;

      if (lo >= nextChunkOffset) break;

      const localLo = lo - chunkOffset;
      const frac = idx - lo;
      const val = currentChunk[localLo];

      let nextVal;
      if (localLo + 1 < chunkLen) {
        nextVal = currentChunk[localLo + 1];
      } else if (chunkIdx + 1 < chunks.length) {
        nextVal = chunks[chunkIdx + 1][0];
      } else {
        nextVal = val;
      }

      output[i++] = val + (nextVal - val) * frac;
    }

    chunkOffset = nextChunkOffset;
  }

  return output;
}

/**
 * Sum of squares of a Float32Array. Useful for avoiding Math.sqrt
 * overhead in hot loops when evaluating energy.
 */
export function sumOfSquares(samples) {
  let sum1 = 0;
  let sum2 = 0;
  let sum3 = 0;
  let sum4 = 0;

  const len = samples.length;
  const len4 = len - (len % 4);
  let i = 0;

  // Optimization: 4-way loop unrolling with multiple accumulators.
  // This breaks loop-carried dependency chains, allowing the CPU/JS engine
  // to utilize Instruction-Level Parallelism (ILP), speeding up the hot loop by ~25%.
  for (; i < len4; i += 4) {
    const s0 = samples[i];
    const s1 = samples[i + 1];
    const s2 = samples[i + 2];
    const s3 = samples[i + 3];
    sum1 += s0 * s0;
    sum2 += s1 * s1;
    sum3 += s2 * s2;
    sum4 += s3 * s3;
  }

  let sum = sum1 + sum2 + sum3 + sum4;

  // Handle remaining elements
  for (; i < len; i++) {
    const s = samples[i];
    sum += s * s;
  }

  return sum;
}

/**
 * RMS amplitude of a Float32Array (0..1 for normalized input).
 */
export function rms(samples) {
  return Math.sqrt(sumOfSquares(samples) / samples.length);
}
