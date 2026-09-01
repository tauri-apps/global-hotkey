---
"global-hotkey": patch
---

On Windows, fix `Ctrl+Pause` hotkeys never triggering by registering them as `VK_CANCEL` (Break), which is the virtual key the keyboard driver reports when `Pause` is pressed with `Ctrl` held.
