---
"global-hotkey": patch
---

On macOS, register hotkeys exclusively, matching Windows and X11. Registering a hotkey that another app already holds exclusively now fails with `Error::AlreadyRegistered` instead of succeeding and never firing, and registering a hotkey that is already taken (including one this process already registered) returns `Error::AlreadyRegistered` instead of `Error::FailedToRegister`.
