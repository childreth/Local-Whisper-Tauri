## 2026-06-09 - JavaScript Audio Processing Hot Loops
**Learning:** `DataView.setInt16` inside a hot loop is a significant bottleneck for typed array writes in JS engines, performing roughly 50-60% slower than writing directly to an `Int16Array`.
**Action:** When converting generic audio bytes, write a little-endian fast path that writes directly to an `Int16Array` view and returns the underlying `Uint8Array` buffer. Also, avoid `Math.min` in tight inner loops (like resampling) by caching bounds and using ternaries.
