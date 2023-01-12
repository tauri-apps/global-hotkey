global_hotkey lets you register Global HotKeys for Desktop Applications.

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

## Platforms-supported:

- Windows
- macOS
- Linux (X11 Only)

## License

Apache-2.0/MIT
