// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! HotKeys describe keyboard global shortcuts.
//!
//! [`HotKey`s](crate::hotkey::HotKey) are used to define a keyboard shortcut consisting
//! of an optional combination of modifier keys (provided by [`Modifiers`](crate::hotkey::Modifiers)) and
//! one key ([`Code`](crate::hotkey::Code)).
//!
//! # Examples
//! They can be created directly
//! ```no_run
//! # use global_hotkey::hotkey::{HotKey, Modifiers, Code};
//! let hotkey = HotKey::new(Some(Modifiers::SHIFT), Code::KeyQ);
//! let hotkey_without_mods = HotKey::new(None, Code::KeyQ);
//! ```
//! or from `&str`, note that all modifiers
//! have to be listed before the non-modifier key, `shift+alt+KeyQ` is legal,
//! whereas `shift+q+alt` is not.
//! ```no_run
//! # use global_hotkey::hotkey::{HotKey};
//! let hotkey: HotKey = "shift+alt+KeyQ".parse().unwrap();
//! # // This assert exists to ensure a test breaks once the
//! # // statement above about ordering is no longer valid.
//! # assert!("shift+KeyQ+alt".parse::<HotKey>().is_err());
//! ```
//!

pub use keyboard_types::{Code, Modifiers};
use std::{borrow::Borrow, hash::Hash, str::FromStr};

use crate::counter::Counter;

static COUNTER: Counter = Counter::new();

/// A keyboard shortcut that consists of an optional combination
/// of modifier keys (provided by [`Modifiers`](crate::hotkey::Modifiers)) and
/// one key ([`Code`](crate::hotkey::Code)).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct HotKey {
    pub(crate) mods: Modifiers,
    pub(crate) key: Code,
    id: u32,
}

impl HotKey {
    /// Creates a new hotkey to define keyboard shortcuts throughout your application.
    /// Only [`Modifiers::ALT`], [`Modifiers::SHIFT`], [`Modifiers::CONTROL`], and [`Modifiers::META`]/[`Modifiers::SUPER`]
    pub fn new(mods: Option<Modifiers>, key: Code) -> Self {
        Self {
            mods: mods.unwrap_or_else(Modifiers::empty),
            key,
            id: COUNTER.next(),
        }
    }

    /// Returns the id associated with this HotKey
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns `true` if this [`Code`] and [`Modifiers`] matches this `hotkey`.
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Code>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        let base_mods = Modifiers::SHIFT
            | Modifiers::CONTROL
            | Modifiers::ALT
            | Modifiers::META
            | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == *key
    }
}

// HotKey::from_str is available to be backward
// compatible with tauri and it also open the option
// to generate hotkey from string
impl FromStr for HotKey {
    type Err = crate::Error;
    fn from_str(hotkey_string: &str) -> Result<Self, Self::Err> {
        parse_hotkey(hotkey_string)
    }
}

fn parse_hotkey(hotkey: &str) -> crate::Result<HotKey> {
    let tokens = hotkey.split('+').collect::<Vec<&str>>();

    let mut mods = Modifiers::empty();
    let mut key = None;

    match tokens.len() {
        // single key hotkey
        1 => {
            key = Some(parse_key(tokens[0])?);
        }
        // modifiers and key comobo hotkey
        _ => {
            for raw in tokens {
                let token = raw.trim();

                if token.is_empty() {
                    return Err(crate::Error::EmptyHotKeyToken(hotkey.to_string()));
                }

                if key.is_some() {
                    // At this point we have parsed the modifiers and a main key, so by reaching
                    // this code, the function either received more than one main key or
                    //  the hotkey is not in the right order
                    // examples:
                    // 1. "Ctrl+Shift+C+A" => only one main key should be allowd.
                    // 2. "Ctrl+C+Shift" => wrong order
                    return Err(crate::Error::UnexpectedHotKeyFormat(hotkey.to_string()));
                }

                match token.to_uppercase().as_str() {
                    "OPTION" | "ALT" => {
                        mods.set(Modifiers::ALT, true);
                    }
                    "CONTROL" | "CTRL" => {
                        mods.set(Modifiers::CONTROL, true);
                    }
                    "COMMAND" | "CMD" | "SUPER" => {
                        mods.set(Modifiers::META, true);
                    }
                    "SHIFT" => {
                        mods.set(Modifiers::SHIFT, true);
                    }
                    "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
                        #[cfg(target_os = "macos")]
                        mods.set(Modifiers::META, true);
                        #[cfg(not(target_os = "macos"))]
                        mods.set(Modifiers::CONTROL, true);
                    }
                    _ => {
                        key = Some(parse_key(token)?);
                    }
                }
            }
        }
    }

    Ok(HotKey {
        // safe to unwrap, will always be some
        // as we made sure to return an error earlier
        key: key.unwrap(),
        mods,
        id: COUNTER.next(),
    })
}

fn parse_key(key: &str) -> crate::Result<Code> {
    use Code::*;
    match key.to_uppercase().as_str() {
        "BACKQUOTE" | "`" => Ok(Backquote),
        "BACKSLASH" | "\\" => Ok(Backslash),
        "BRACKETLEFT" | "[" => Ok(BracketLeft),
        "BRACKETRIGHT" | "]" => Ok(BracketRight),
        "COMMA" | "," => Ok(Comma),
        "DIGIT0" | "0" => Ok(Digit0),
        "DIGIT1" | "1" => Ok(Digit1),
        "DIGIT2" | "2" => Ok(Digit2),
        "DIGIT3" | "3" => Ok(Digit3),
        "DIGIT4" | "4" => Ok(Digit4),
        "DIGIT5" | "5" => Ok(Digit5),
        "DIGIT6" | "6" => Ok(Digit6),
        "DIGIT7" | "7" => Ok(Digit7),
        "DIGIT8" | "8" => Ok(Digit8),
        "DIGIT9" | "9" => Ok(Digit9),
        "EQUAL" | "=" => Ok(Equal),
        "KEYA" | "A" => Ok(KeyA),
        "KEYB" | "B" => Ok(KeyB),
        "KEYC" | "C" => Ok(KeyC),
        "KEYD" | "D" => Ok(KeyD),
        "KEYE" | "E" => Ok(KeyE),
        "KEYF" | "F" => Ok(KeyF),
        "KEYG" | "G" => Ok(KeyG),
        "KEYH" | "H" => Ok(KeyH),
        "KEYI" | "I" => Ok(KeyI),
        "KEYJ" | "J" => Ok(KeyJ),
        "KEYK" | "K" => Ok(KeyK),
        "KEYL" | "L" => Ok(KeyL),
        "KEYM" | "M" => Ok(KeyM),
        "KEYN" | "N" => Ok(KeyN),
        "KEYO" | "O" => Ok(KeyO),
        "KEYP" | "P" => Ok(KeyP),
        "KEYQ" | "Q" => Ok(KeyQ),
        "KEYR" | "R" => Ok(KeyR),
        "KEYS" | "S" => Ok(KeyS),
        "KEYT" | "T" => Ok(KeyT),
        "KEYU" | "U" => Ok(KeyU),
        "KEYV" | "V" => Ok(KeyV),
        "KEYW" | "W" => Ok(KeyW),
        "KEYX" | "X" => Ok(KeyX),
        "KEYY" | "Y" => Ok(KeyY),
        "KEYZ" | "Z" => Ok(KeyZ),
        "MINUS" | "-" => Ok(Minus),
        "PERIOD" | "." => Ok(Period),
        "QUOTE" | "'" => Ok(Quote),
        "SEMICOLON" | ";" => Ok(Semicolon),
        "SLASH" | "/" => Ok(Slash),
        "BACKSPACE" => Ok(Backspace),
        "CAPSLOCK" => Ok(CapsLock),
        "ENTER" => Ok(Enter),
        "SPACE" => Ok(Space),
        "TAB" => Ok(Tab),
        "DELETE" => Ok(Delete),
        "END" => Ok(End),
        "HOME" => Ok(Home),
        "INSERT" => Ok(Insert),
        "PAGEDOWN" => Ok(PageDown),
        "PAGEUP" => Ok(PageUp),
        "PRINTSCREEN" => Ok(PrintScreen),
        "SCROLLLOCK" => Ok(ScrollLock),
        "ARROWDOWN" | "DOWN" => Ok(ArrowDown),
        "ARROWLEFT" | "LEFT" => Ok(ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Ok(ArrowRight),
        "ARROWUP" | "UP" => Ok(ArrowUp),
        "NUMLOCK" => Ok(NumLock),
        "NUMPAD0" | "NUM0" => Ok(Numpad0),
        "NUMPAD1" | "NUM1" => Ok(Numpad1),
        "NUMPAD2" | "NUM2" => Ok(Numpad2),
        "NUMPAD3" | "NUM3" => Ok(Numpad3),
        "NUMPAD4" | "NUM4" => Ok(Numpad4),
        "NUMPAD5" | "NUM5" => Ok(Numpad5),
        "NUMPAD6" | "NUM6" => Ok(Numpad6),
        "NUMPAD7" | "NUM7" => Ok(Numpad7),
        "NUMPAD8" | "NUM8" => Ok(Numpad8),
        "NUMPAD9" | "NUM9" => Ok(Numpad9),
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => Ok(NumpadAdd),
        "NUMPADDECIMAL" | "NUMDECIMAL" => Ok(NumpadDecimal),
        "NUMPADDIVIDE" | "NUMDIVIDE" => Ok(NumpadDivide),
        "NUMPADENTER" | "NUMENTER" => Ok(NumpadEnter),
        "NUMPADEQUAL" | "NUMEQUAL" => Ok(NumpadEqual),
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => Ok(NumpadMultiply),
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => Ok(NumpadSubtract),
        "ESCAPE" | "ESC" => Ok(Escape),
        "F1" => Ok(F1),
        "F2" => Ok(F2),
        "F3" => Ok(F3),
        "F4" => Ok(F4),
        "F5" => Ok(F5),
        "F6" => Ok(F6),
        "F7" => Ok(F7),
        "F8" => Ok(F8),
        "F9" => Ok(F9),
        "F10" => Ok(F10),
        "F11" => Ok(F11),
        "F12" => Ok(F12),
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => Ok(AudioVolumeDown),
        "AUDIOVOLUMEUP" | "VOLUMEUP" => Ok(AudioVolumeUp),
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => Ok(AudioVolumeMute),
        "F13" => Ok(F13),
        "F14" => Ok(F14),
        "F15" => Ok(F15),
        "F16" => Ok(F16),
        "F17" => Ok(F17),
        "F18" => Ok(F18),
        "F19" => Ok(F19),
        "F20" => Ok(F20),
        "F21" => Ok(F21),
        "F22" => Ok(F22),
        "F23" => Ok(F23),
        "F24" => Ok(F24),

        _ => Err(crate::Error::UnrecognizedHotKeyCode(key.to_string())),
    }
}

#[test]
fn test_parse_hotkey() {
    macro_rules! assert_parse_hotkey {
        ($key:literal, $lrh:expr) => {
            let r = parse_hotkey($key).unwrap();
            let l = $lrh;
            assert_eq!(r.mods, l.mods);
            assert_eq!(r.key, l.key);
        };
    }

    assert_parse_hotkey!(
        "KeyX",
        HotKey {
            mods: Modifiers::empty(),
            key: Code::KeyX,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "CTRL+KeyX",
        HotKey {
            mods: Modifiers::CONTROL,
            key: Code::KeyX,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "SHIFT+KeyC",
        HotKey {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "SHIFT+KeyC",
        HotKey {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "super+ctrl+SHIFT+alt+ArrowUp",
        HotKey {
            mods: Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
            key: Code::ArrowUp,
            id: 0,
        }
    );
    assert_parse_hotkey!(
        "Digit5",
        HotKey {
            mods: Modifiers::empty(),
            key: Code::Digit5,
            id: 0,
        }
    );
    assert_parse_hotkey!(
        "KeyG",
        HotKey {
            mods: Modifiers::empty(),
            key: Code::KeyG,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "SHiFT+F12",
        HotKey {
            mods: Modifiers::SHIFT,
            key: Code::F12,
            id: 0,
        }
    );

    assert_parse_hotkey!(
        "CmdOrCtrl+Space",
        HotKey {
            #[cfg(target_os = "macos")]
            mods: Modifiers::META,
            #[cfg(not(target_os = "macos"))]
            mods: Modifiers::CONTROL,
            key: Code::Space,
            id: 0,
        }
    );
}
