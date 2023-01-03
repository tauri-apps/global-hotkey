// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, ptr};

use crossbeam_channel::{unbounded, Sender};
use keyboard_types::{Code, Modifiers};
use x11_dl::{keysym, xlib};

use {crate::hotkey::HotKey, GlobalHotKeyEvent};

enum ThreadMessage {
    RegisterHotKey(HotKey, Sender<crate::Result<()>>),
    UnRegisterHotKey(HotKey, Sender<crate::Result<()>>),
    DropThread,
}

pub struct GlobalHotKeyManager {
    thread_tx: Sender<ThreadMessage>,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        let (thread_tx, thread_rx) = unbounded();

        std::thread::spawn(move || {
            //                           mods, key    id,  repeating
            let mut hotkeys = HashMap::<(u32, u32), (u32, bool)>::new();
            let xlib = xlib::Xlib::open().unwrap();
            unsafe {
                let display = (xlib.XOpenDisplay)(ptr::null());
                let root = (xlib.XDefaultRootWindow)(display);

                // Only trigger key release at end of repeated keys
                let mut supported_rtrn: i32 = 0;
                (xlib.XkbSetDetectableAutoRepeat)(display, 1, &mut supported_rtrn);

                (xlib.XSelectInput)(display, root, xlib::KeyPressMask);
                let mut event: xlib::XEvent = std::mem::zeroed();

                loop {
                    if (xlib.XPending)(display) > 0 {
                        (xlib.XNextEvent)(display, &mut event);
                        match event.get_type() {
                            e if matches!(e, xlib::KeyPress | xlib::KeyRelease) => {
                                let keycode = event.key.keycode;
                                // X11 sends masks for Lock keys also and we only care about the 4 below
                                let modifiers = event.key.state
                                    & (xlib::ControlMask
                                        | xlib::ShiftMask
                                        | xlib::Mod4Mask
                                        | xlib::Mod1Mask);

                                if let Some((id, repeating)) =
                                    hotkeys.get_mut(&(modifiers, keycode))
                                {
                                    match (e, *repeating) {
                                        (xlib::KeyPress, false) => {
                                            GlobalHotKeyEvent::send(GlobalHotKeyEvent { id: *id });
                                            *repeating = true;
                                        }
                                        (xlib::KeyRelease, true) => {
                                            *repeating = false;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // XGrabKey works only with the exact state (modifiers)
                    // and since X11 considers NumLock, ScrollLock and CapsLock a modifier when it is ON,
                    // we also need to register our shortcut combined with these extra modifiers as well
                    const IGNORED_MODS: [u32; 4] = [
                        0,              // modifier only
                        xlib::Mod2Mask, // NumLock
                        xlib::LockMask, // CapsLock
                        xlib::Mod2Mask | xlib::LockMask,
                    ];

                    if let Ok(msg) = thread_rx.try_recv() {
                        match msg {
                            ThreadMessage::RegisterHotKey(hotkey, tx) => {
                                let (modifiers, key) = (
                                    modifiers_to_x11_mods(hotkey.mods),
                                    keycode_to_x11_scancode(hotkey.key),
                                );

                                if let Some(key) = key {
                                    let keycode = (xlib.XKeysymToKeycode)(display, key as _);

                                    let mut errored = false;

                                    for m in IGNORED_MODS {
                                        let result = (xlib.XGrabKey)(
                                            display,
                                            keycode as _,
                                            modifiers | m,
                                            root,
                                            0,
                                            xlib::GrabModeAsync,
                                            xlib::GrabModeAsync,
                                        );

                                        if result == xlib::BadAccess as _ {
                                            errored = true;

                                            let _ = tx
                                                .send(Err(crate::Error::AlreadyRegistered(hotkey)));

                                            for m in IGNORED_MODS {
                                                (xlib.XUngrabKey)(
                                                    display,
                                                    keycode as _,
                                                    modifiers | m,
                                                    root,
                                                );
                                            }

                                            break;
                                        }
                                    }

                                    if !errored {
                                        if hotkeys.contains_key(&(modifiers, keycode as _)) {
                                            let _ = tx
                                                .send(Err(crate::Error::AlreadyRegistered(hotkey)));
                                        } else {
                                            hotkeys.insert(
                                                (modifiers, keycode as _),
                                                (hotkey.id(), false),
                                            );
                                        }

                                        let _ = tx.send(Ok(()));
                                    }
                                } else {
                                    let _ = tx
                                    .send(Err(crate::Error::FailedToRegister(format!(
                                        "Unable to register accelerator (unknown scancode for this key: {}).",
                                        hotkey.key
                                    ))));
                                }
                            }
                            ThreadMessage::UnRegisterHotKey(hotkey, tx) => {
                                let (modifiers, key) = (
                                    modifiers_to_x11_mods(hotkey.mods),
                                    keycode_to_x11_scancode(hotkey.key),
                                );

                                if let Some(key) = key {
                                    let keycode = (xlib.XKeysymToKeycode)(display, key as _);

                                    for m in IGNORED_MODS {
                                        (xlib.XUngrabKey)(
                                            display,
                                            keycode as _,
                                            modifiers | m,
                                            root,
                                        );
                                    }

                                    hotkeys.remove(&(modifiers, keycode as _));

                                    let _ = tx.send(Ok(()));
                                } else {
                                    // send back error
                                }
                            }
                            ThreadMessage::DropThread => {
                                (xlib.XCloseDisplay)(display);
                                return;
                            }
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            };
        });

        Ok(Self { thread_tx })
    }

    pub fn register(&self, hotkey: HotKey) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::RegisterHotKey(hotkey, tx));

        if let Ok(result) = rx.recv() {
            result?;
        }

        Ok(())
    }

    pub fn unregister(&self, hotkey: HotKey) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::UnRegisterHotKey(hotkey, tx));

        if let Ok(result) = rx.recv() {
            result?;
        }

        Ok(())
    }
}

impl Drop for GlobalHotKeyManager {
    fn drop(&mut self) {
        let _ = self.thread_tx.send(ThreadMessage::DropThread);
    }
}

fn keycode_to_x11_scancode(key: Code) -> Option<u32> {
    Some(match key {
        Code::KeyA => 'A' as u32,
        Code::KeyB => 'B' as u32,
        Code::KeyC => 'C' as u32,
        Code::KeyD => 'D' as u32,
        Code::KeyE => 'E' as u32,
        Code::KeyF => 'F' as u32,
        Code::KeyG => 'G' as u32,
        Code::KeyH => 'H' as u32,
        Code::KeyI => 'I' as u32,
        Code::KeyJ => 'J' as u32,
        Code::KeyK => 'K' as u32,
        Code::KeyL => 'L' as u32,
        Code::KeyM => 'M' as u32,
        Code::KeyN => 'N' as u32,
        Code::KeyO => 'O' as u32,
        Code::KeyP => 'P' as u32,
        Code::KeyQ => 'Q' as u32,
        Code::KeyR => 'R' as u32,
        Code::KeyS => 'S' as u32,
        Code::KeyT => 'T' as u32,
        Code::KeyU => 'U' as u32,
        Code::KeyV => 'V' as u32,
        Code::KeyW => 'W' as u32,
        Code::KeyX => 'X' as u32,
        Code::KeyY => 'Y' as u32,
        Code::KeyZ => 'Z' as u32,
        Code::Backslash => keysym::XK_backslash,
        Code::BracketLeft => keysym::XK_bracketleft,
        Code::BracketRight => keysym::XK_bracketright,
        Code::Comma => keysym::XK_comma,
        Code::Digit0 => '0' as u32,
        Code::Digit1 => '1' as u32,
        Code::Digit2 => '2' as u32,
        Code::Digit3 => '3' as u32,
        Code::Digit4 => '4' as u32,
        Code::Digit5 => '5' as u32,
        Code::Digit6 => '6' as u32,
        Code::Digit7 => '7' as u32,
        Code::Digit8 => '8' as u32,
        Code::Digit9 => '9' as u32,
        Code::Equal => keysym::XK_equal,
        Code::IntlBackslash => keysym::XK_backslash,
        Code::Minus => keysym::XK_minus,
        Code::Period => keysym::XK_period,
        Code::Quote => keysym::XK_leftsinglequotemark,
        Code::Semicolon => keysym::XK_semicolon,
        Code::Slash => keysym::XK_slash,
        Code::Backspace => keysym::XK_BackSpace,
        Code::CapsLock => keysym::XK_Caps_Lock,
        Code::Enter => keysym::XK_Return,
        Code::Space => keysym::XK_space,
        Code::Tab => keysym::XK_Tab,
        Code::Delete => keysym::XK_Delete,
        Code::End => keysym::XK_End,
        Code::Home => keysym::XK_Home,
        Code::Insert => keysym::XK_Insert,
        Code::PageDown => keysym::XK_Page_Down,
        Code::PageUp => keysym::XK_Page_Up,
        Code::ArrowDown => keysym::XK_Down,
        Code::ArrowLeft => keysym::XK_Left,
        Code::ArrowRight => keysym::XK_Right,
        Code::ArrowUp => keysym::XK_Up,
        Code::Numpad0 => keysym::XK_KP_0,
        Code::Numpad1 => keysym::XK_KP_1,
        Code::Numpad2 => keysym::XK_KP_2,
        Code::Numpad3 => keysym::XK_KP_3,
        Code::Numpad4 => keysym::XK_KP_4,
        Code::Numpad5 => keysym::XK_KP_5,
        Code::Numpad6 => keysym::XK_KP_6,
        Code::Numpad7 => keysym::XK_KP_7,
        Code::Numpad8 => keysym::XK_KP_8,
        Code::Numpad9 => keysym::XK_KP_9,
        Code::NumpadAdd => keysym::XK_KP_Add,
        Code::NumpadDecimal => keysym::XK_KP_Decimal,
        Code::NumpadDivide => keysym::XK_KP_Divide,
        Code::NumpadMultiply => keysym::XK_KP_Multiply,
        Code::NumpadSubtract => keysym::XK_KP_Subtract,
        Code::Escape => keysym::XK_Escape,
        Code::PrintScreen => keysym::XK_Print,
        Code::ScrollLock => keysym::XK_Scroll_Lock,
        Code::Pause => keysym::XF86XK_AudioPlay,
        Code::MediaStop => keysym::XF86XK_AudioStop,
        Code::MediaTrackNext => keysym::XF86XK_AudioNext,
        Code::MediaTrackPrevious => keysym::XF86XK_AudioPrev,
        Code::AudioVolumeDown => keysym::XF86XK_AudioLowerVolume,
        Code::AudioVolumeMute => keysym::XF86XK_AudioMute,
        Code::AudioVolumeUp => keysym::XF86XK_AudioRaiseVolume,
        Code::F1 => keysym::XK_F1,
        Code::F2 => keysym::XK_F2,
        Code::F3 => keysym::XK_F3,
        Code::F4 => keysym::XK_F4,
        Code::F5 => keysym::XK_F5,
        Code::F6 => keysym::XK_F6,
        Code::F7 => keysym::XK_F7,
        Code::F8 => keysym::XK_F8,
        Code::F9 => keysym::XK_F9,
        Code::F10 => keysym::XK_F10,
        Code::F11 => keysym::XK_F11,
        Code::F12 => keysym::XK_F12,

        _ => return None,
    })
}

fn modifiers_to_x11_mods(modifiers: Modifiers) -> u32 {
    let mut x11mods = 0;
    if modifiers.contains(Modifiers::SHIFT) {
        x11mods |= xlib::ShiftMask;
    }
    if modifiers.intersects(Modifiers::SUPER | Modifiers::META) {
        x11mods |= xlib::Mod4Mask;
    }
    if modifiers.contains(Modifiers::ALT) {
        x11mods |= xlib::Mod1Mask;
    }
    if modifiers.contains(Modifiers::CONTROL) {
        x11mods |= xlib::ControlMask;
    }
    x11mods
}
