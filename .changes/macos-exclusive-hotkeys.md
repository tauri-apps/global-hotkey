---
"global-hotkey": patch
---

On macOS, register hotkeys exclusively and return `Error::AlreadyRegistered` when a hotkey is already taken. Previously, registration could succeed even though the hotkey would never fire.
