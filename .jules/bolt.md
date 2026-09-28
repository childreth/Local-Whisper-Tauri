## 2025-01-20 - Optimize AudioWorklet hot loop frame timing
**Learning:** In highly-frequent Javascript loops (such as ~375Hz AudioWorklet callbacks), repeatedly calling `performance.now()` for simple throttling evaluations causes unnecessary CPU overhead.
**Action:** Replace `performance.now()` with raw frame sample count tracking in hot loops. Calculate the sample threshold equivalent using the target time delta and sample rate upfront.
