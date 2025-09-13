use keyboard_types::Code;

use crate::hotkey::HotKey;

pub fn using_wayland() -> bool {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    {
        use std::env;
        return env::var("WAYLAND_DISPLAY").is_ok();
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    )))]
    return false;
}

#[derive(Clone)]
pub struct WlHotKey {
    id: u32,
    description: String,
    preferred_trigger: Option<String>,
}

impl WlHotKey {
    pub fn new<S>(id: u32, description: S, preferred_trigger: Option<HotKey>) -> Self
    where
        S: Into<String>,
    {
        Self {
            id,
            description: description.into(),
            preferred_trigger: preferred_trigger.and_then(hotkey_to_wayland_trigger),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn preferred_trigger(&self) -> Option<&str> {
        self.preferred_trigger.as_deref()
    }
}

fn hotkey_to_wayland_trigger(hotkey: HotKey) -> Option<String> {
    let mut mods = "".to_string();

    if hotkey.mods.ctrl() {
        mods += "CTRL+";
    }
    if hotkey.mods.shift() {
        mods += "SHIFT+";
    }
    if hotkey.mods.alt() {
        mods += "ALT+";
    }
    if hotkey.mods.meta() {
        mods += "LOGO+";
    }

    let keycode = match hotkey.key {
        Code::KeyA => "A",
        Code::KeyB => "B",
        Code::KeyC => "C",
        Code::KeyD => "D",
        Code::KeyE => "E",
        Code::KeyF => "F",
        Code::KeyG => "G",
        Code::KeyH => "H",
        Code::KeyI => "I",
        Code::KeyJ => "J",
        Code::KeyK => "K",
        Code::KeyL => "L",
        Code::KeyM => "M",
        Code::KeyN => "N",
        Code::KeyO => "O",
        Code::KeyP => "P",
        Code::KeyQ => "Q",
        Code::KeyR => "R",
        Code::KeyS => "S",
        Code::KeyT => "T",
        Code::KeyU => "U",
        Code::KeyV => "V",
        Code::KeyW => "W",
        Code::KeyX => "X",
        Code::KeyY => "Y",
        Code::KeyZ => "Z",
        Code::Backslash => "backslash",
        Code::BracketLeft => "bracketleft",
        Code::BracketRight => "bracketright",
        Code::Backquote => "grave",
        Code::Comma => "comma",
        Code::Digit0 => "0",
        Code::Digit1 => "1",
        Code::Digit2 => "2",
        Code::Digit3 => "3",
        Code::Digit4 => "4",
        Code::Digit5 => "5",
        Code::Digit6 => "6",
        Code::Digit7 => "7",
        Code::Digit8 => "8",
        Code::Digit9 => "9",
        Code::Equal => "equal",
        Code::Minus => "minus",
        Code::Period => "period",
        Code::Quote => "apostrophe",
        Code::Semicolon => "semicolon",
        Code::Slash => "slash",
        Code::Backspace => "BackSpace",
        Code::CapsLock => "Caps_Lock",
        Code::Enter => "Return",
        Code::Space => "space",
        Code::Tab => "Tab",
        Code::Delete => "Delete",
        Code::End => "End",
        Code::Home => "Home",
        Code::Insert => "Insert",
        Code::PageDown => "Page_Down",
        Code::PageUp => "Page_Up",
        Code::ArrowDown => "downarrow",
        Code::ArrowLeft => "leftarrow",
        Code::ArrowRight => "rightarrow",
        Code::ArrowUp => "uparrow",
        Code::Numpad0 => "KP_0",
        Code::Numpad1 => "KP_1",
        Code::Numpad2 => "KP_2",
        Code::Numpad3 => "KP_3",
        Code::Numpad4 => "KP_4",
        Code::Numpad5 => "KP_5",
        Code::Numpad6 => "KP_6",
        Code::Numpad7 => "KP_7",
        Code::Numpad8 => "KP_8",
        Code::Numpad9 => "KP_9",
        Code::NumpadAdd => "KP_Add",
        Code::NumpadDecimal => "KP_Decimal",
        Code::NumpadDivide => "KP_Divide",
        Code::NumpadMultiply => "KP_Multiply",
        Code::NumpadSubtract => "KP_Subtract",
        Code::Escape => "Escape",
        Code::PrintScreen => "Print",
        Code::ScrollLock => "Scroll_Lock",
        Code::NumLock => "Num_lock",
        Code::F1 => "F1",
        Code::F2 => "F2",
        Code::F3 => "F3",
        Code::F4 => "F4",
        Code::F5 => "F5",
        Code::F6 => "F6",
        Code::F7 => "F7",
        Code::F8 => "F8",
        Code::F9 => "F9",
        Code::F10 => "F10",
        Code::F11 => "F11",
        Code::F12 => "F12",
        Code::AudioVolumeDown => "XF86AudioLowerVolume",
        Code::AudioVolumeMute => "XF86AudioMute",
        Code::AudioVolumeUp => "XF86AudioRaiseVolume",
        Code::MediaPlay => "XF86AudioPlay",
        Code::MediaPause => "XF86AudioPause",
        Code::MediaStop => "XF86AudioStop",
        Code::MediaTrackNext => "XF86AudioNext",
        Code::MediaTrackPrevious => "XF86AudioPrev",
        Code::Pause => "Pause",
        _ => return None,
    };

    Some(mods + keycode)
}
