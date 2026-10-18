---
"global-hotkey": patch
---

On Linux (X11), fix registering hotkeys that use `Code::Quote` failing with `Unable to find keycode for key: Quote` by mapping it to the `apostrophe` keysym instead of `leftsinglequotemark`.
