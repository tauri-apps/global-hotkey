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

fn parse_hotkey(hotkey_string: &str) -> crate::Result<HotKey> {
    let mut mods = Modifiers::empty();
    let mut key = Code::Unidentified;

    let mut split = hotkey_string.split('+');
    let len = split.clone().count();
    let parse_key = |token: &str| -> crate::Result<Code> {
        if let Ok(code) = Code::from_str(token) {
            match code {
                Code::Unidentified => Err(crate::Error::HotKeyParseError(format!(
                    "Couldn't identify \"{}\" as a valid `Code`",
                    token
                ))),
                _ => Ok(code),
            }
        } else {
            Err(crate::Error::HotKeyParseError(format!(
                "Couldn't identify \"{}\" as a valid `Code`",
                token
            )))
        }
    };

    if len == 1 {
        let token = split.next().unwrap();
        key = parse_key(token)?;
    } else {
        for raw in split {
            let token = raw.trim().to_string();
            if token.is_empty() {
                return Err(crate::Error::HotKeyParseError(
                    "Unexpected empty token while parsing hotkey".into(),
                ));
            }

            if key != Code::Unidentified {
                // at this point we already parsed the modifiers and found a main key but
                // the function received more then one main key or it is not in the right order
                // examples:
                // 1. "Ctrl+Shift+C+A" => only one main key should be allowd.
                // 2. "Ctrl+C+Shift" => wrong order
                return Err(crate::Error::HotKeyParseError(format!(
                    "Unexpected hotkey string format: \"{}\"",
                    hotkey_string
                )));
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
                    key = parse_key(token.as_str())?;
                }
            }
        }
    }

    Ok(HotKey {
        key,
        mods,
        id: COUNTER.next(),
    })
}

#[test]
fn test_parse_hotkey() {
    assert_eq!(
        parse_hotkey("KeyX").unwrap(),
        HotKey {
            mods: Modifiers::empty(),
            key: Code::KeyX,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("CTRL+KeyX").unwrap(),
        HotKey {
            mods: Modifiers::CONTROL,
            key: Code::KeyX,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("SHIFT+KeyC").unwrap(),
        HotKey {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("CTRL+KeyZ").unwrap(),
        HotKey {
            mods: Modifiers::CONTROL,
            key: Code::KeyZ,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("super+ctrl+SHIFT+alt+ArrowUp").unwrap(),
        HotKey {
            mods: Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
            key: Code::ArrowUp,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("Digit5").unwrap(),
        HotKey {
            mods: Modifiers::empty(),
            key: Code::Digit5,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("KeyG").unwrap(),
        HotKey {
            mods: Modifiers::empty(),
            key: Code::KeyG,
            id: 0,
        }
    );

    let hotkey = parse_hotkey("+G");
    assert!(hotkey.is_err());

    let hotkey = parse_hotkey("SHGSH+G");
    assert!(hotkey.is_err());

    assert_eq!(
        parse_hotkey("SHiFT+F12").unwrap(),
        HotKey {
            mods: Modifiers::SHIFT,
            key: Code::F12,
            id: 0,
        }
    );
    assert_eq!(
        parse_hotkey("CmdOrCtrl+Space").unwrap(),
        HotKey {
            #[cfg(target_os = "macos")]
            mods: Modifiers::META,
            #[cfg(not(target_os = "macos"))]
            mods: Modifiers::CONTROL,
            key: Code::Space,
            id: 0,
        }
    );

    let hotkey = parse_hotkey("CTRL+");
    assert!(hotkey.is_err());
}
