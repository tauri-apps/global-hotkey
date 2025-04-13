global_hotkey lets you register Global HotKeys for Desktop Applications.

## Platforms-supported:

- Windows
- macOS
- Linux (X11 Only)

## Platform-specific notes:

- On Windows a win32 event loop must be running on the thread. It doesn't need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.
- On macOS, an event loop must be running on the main thread so you also need to create the global hotkey manager on the main thread.
- On Linux X11, if attempted to register a hotkey which is already registered by another application, the error will be reported on process-wide Xlib error handler, more information: https://www.remlab.net/op/xlib.shtml
  - For winit, `winit` feature is available which captures global error and properly returns it in `register()` response Result, otherwise the error will result in panic in unrelated place

## Example

```rs
use global_hotkey::{GlobalHotKeyManager, hotkey::{HotKey, Modifiers, Code}};

// initialize the hotkeys manager
let manager = GlobalHotKeyManager::new().unwrap();

// construct the hotkey
let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyD);

// register it
manager.register(hotkey);
```

## Processing global hotkey events

You can also listen for the menu events using `GlobalHotKeyEvent::receiver` to get events for the hotkey pressed events.

```rs
use global_hotkey::GlobalHotKeyEvent;

if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
    println!("{:?}", event);
}
```

## License

Apache-2.0/MIT
