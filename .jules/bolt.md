## 2025-01-20 - Optimize AudioWorklet hot loop frame timing
**Learning:** In highly-frequent Javascript loops (such as ~375Hz AudioWorklet callbacks), repeatedly calling `performance.now()` for simple throttling evaluations causes unnecessary CPU overhead.
**Action:** Replace `performance.now()` with raw frame sample count tracking in hot loops. Calculate the sample threshold equivalent using the target time delta and sample rate upfront.

## 2025-01-22 - AudioWorklet block copy fast-path
**Learning:** When batching small AudioWorklet blocks (e.g., 128 samples) into a larger buffer (e.g., 2048 samples) using a while loop, the vast majority of blocks (e.g., 15 out of 16) perfectly fit without crossing the batch boundary.
**Action:** Always add a fast-path condition (e.g., `if (offset + channelLength <= batchSize)`) to bypass the while loop entirely, allowing a single native `buffer.set()` call without allocating intermediate `.subarray()` views.

## 2025-01-26 - Optimize Svelte store updates for large arrays
**Learning:** When updating a single element in a large Svelte array store, using `.map()` creates a new object allocation for every unmodified element, causing unnecessary GC pressure.
**Action:** Use `.findIndex()` to locate the specific element, create a shallow copy of the array with `[...arr]`, and update only that specific index to preserve object references and drastically reduce allocation overhead.

## 2025-02-06 - Prevent denormal math slowdowns in exponential decay loops
**Learning:** In high-frequency UI animation loops that use mathematical exponential decay (e.g., `level *= 0.85`), the floating-point values eventually approach zero and enter the subnormal (denormal) range. Processing these subnormal numbers causes massive CPU performance penalties in many architectures.
**Action:** Always clamp or flush imperceptibly small decay values to exactly zero (e.g., `if (level < 1e-8) level = 0;`) inside the loop. This prevents the CPU from thrashing on denormal arithmetic and drastically reduces background CPU usage when an animation approaches rest.

## 2025-02-12 - Fractional downsampling unrolling
**Learning:** In JavaScript fractional downsampling (linear interpolation) hot loops, applying 8-way loop unrolling to the innermost `while` loop (processing 8 samples per bounds check) significantly reduces bounds-checking overhead and branch mispredictions, improving performance on large buffers. Furthermore, when performing linear interpolation in V8/Node.js hot loops, inlining the look-ahead array access directly into the mathematical expression (e.g., `chunk[lo + 1] - val`) can outperform caching it in a local variable, as it avoids unnecessary local assignment overhead.
**Action:** Unroll tight mathematical DSP `while` loops processing float arrays and inline sequential array element reads where readability is not drastically impacted to maximize V8 JIT performance.
