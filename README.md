global_hotkey lets you register Global HotKeys for Desktop Applications.

## Platforms-supported:

- Windows
- macOS
- Linux (X11 and Wayland)

## Wayland Support

On Wayland, global hotkeys are registered via the [XDG GlobalShortcuts portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html). Desktop environment support:

| Desktop Environment | Minimum Version | Status    |
| ------------------- | --------------- | --------- |
| KDE Plasma          | 5.27+           | Supported |
| GNOME               | 48+             | Supported |
| Hyprland            | 0.20+           | Supported |

If the GlobalShortcuts portal is unavailable (e.g. GNOME < 48 or older KDE), the X11 backend is used automatically as a fallback via XWayland. Set `GDK_BACKEND=x11` to force the X11 backend explicitly.

Every `register`/`unregister` call rebinds the whole shortcut set through the portal (which may prompt the user on some compositors), so prefer `register_all` when registering multiple hotkeys.

Set the `GLOBAL_HOTKEY_APP_ID` environment variable to your application's ID for proper D-Bus registration (falls back to `FLATPAK_ID` or `com.global-hotkey.app`).

## Platform-specific notes:

- On Windows a win32 event loop must be running on the thread. It doesn't need to be the main thread but you have to create the global hotkey manager on the same thread as the event loop.
- On macOS, an event loop must be running on the main thread so you also need to create the global hotkey manager on the main thread.
- On Linux (Wayland), a tokio multi-thread runtime is used internally for the D-Bus event loop.
- On Linux (X11), no special event loop requirements.

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
