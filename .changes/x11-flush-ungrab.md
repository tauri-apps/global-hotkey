---
"global-hotkey": "patch"
---

On X11, flush the connection after `ungrab_key` in `unregister_hotkey` so the ungrab takes effect immediately. Previously the ungrab request stayed buffered (the event loop polls but never flushes, and unlike `register` there is no `check()` round-trip to flush implicitly), leaving the key grabbed system-wide until the next request that waits for a reply.
