---
"global-hotkey": "minor"
---

Refactored the errors when parsing accelerator from string:

- Added `HotKeyParseError` error enum.
- Removed `Error::UnrecognizedHotKeyCode` enum variant
- Removed `Error::EmptyHotKeyToken` enum variant
- Removed `Error::UnexpectedHotKeyFormat` enum variant
- Changed `Error::HotKeyParseError` inner value from `String` to the newly added `HotKeyParseError` enum.
