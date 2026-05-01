---
"global-hotkey": patch
---

On Windows, sleep 50ms in the release-detection loop to avoid burning a CPU core for the whole hold duration (e.g. push-to-talk).
