## 2025-01-20 - Optimize AudioWorklet hot loop frame timing
**Learning:** In highly-frequent Javascript loops (such as ~375Hz AudioWorklet callbacks), repeatedly calling `performance.now()` for simple throttling evaluations causes unnecessary CPU overhead.
**Action:** Replace `performance.now()` with raw frame sample count tracking in hot loops. Calculate the sample threshold equivalent using the target time delta and sample rate upfront.

## 2025-01-22 - AudioWorklet block copy fast-path
**Learning:** When batching small AudioWorklet blocks (e.g., 128 samples) into a larger buffer (e.g., 2048 samples) using a while loop, the vast majority of blocks (e.g., 15 out of 16) perfectly fit without crossing the batch boundary.
**Action:** Always add a fast-path condition (e.g., `if (offset + channelLength <= batchSize)`) to bypass the while loop entirely, allowing a single native `buffer.set()` call without allocating intermediate `.subarray()` views.
