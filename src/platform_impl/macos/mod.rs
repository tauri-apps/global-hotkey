use std::{cell::RefCell, collections::HashMap, ffi::c_void};

use keyboard_types::{Code, Modifiers};

use crate::{hotkey::HotKey, GlobalHotKeyEvent};

use self::ffi::{
    kEventClassKeyboard, kEventHotKeyPressed, kEventParamDirectObject, noErr, typeEventHotKeyID,
    EventHandlerCallRef, EventHandlerRef, EventHotKeyID, EventHotKeyRef, EventRef, EventTypeSpec,
    GetApplicationEventTarget, GetEventParameter, InstallEventHandler, OSStatus,
    RegisterEventHotKey, RemoveEventHandler, UnregisterEventHotKey,
};

mod ffi;

pub struct GlobalHotKeyManager {
    event_handler_ptr: EventHandlerRef,
    hotkeys: RefCell<HashMap<u32, HotKeyWrapper>>,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        let event_type = EventTypeSpec {
            eventClass: kEventClassKeyboard,
            eventKind: kEventHotKeyPressed,
        };

        let ptr = unsafe {
            let mut handler_ref: EventHandlerRef = std::mem::zeroed();

            let result = InstallEventHandler(
                GetApplicationEventTarget(),
                Some(hotkey_handler),
                1,
                &event_type,
                std::ptr::null_mut(),
                &mut handler_ref,
            );

            if result != noErr as _ {
                return Err(crate::Error::OsError(std::io::Error::last_os_error()));
            }

            handler_ref
        };

        Ok(Self {
            event_handler_ptr: ptr,
            hotkeys: RefCell::new(HashMap::new()),
        })
    }

    pub fn register(&self, hotkey: HotKey) -> crate::Result<()> {
        let mut mods: u32 = 0;
        if hotkey.mods.contains(Modifiers::SHIFT) {
            mods |= 512;
        }
        if hotkey.mods.intersects(Modifiers::SUPER | Modifiers::META) {
            mods |= 256;
        }
        if hotkey.mods.contains(Modifiers::ALT) {
            mods |= 2048;
        }
        if hotkey.mods.contains(Modifiers::CONTROL) {
            mods |= 4096;
        }

        if let Some(scan_code) = key_to_scancode(hotkey.key) {
            let hotkey_id = EventHotKeyID {
                id: hotkey.id(),
                signature: {
                    let mut res: u32 = 0;
                    // can't find a resource for "htrs" so we construct it manually
                    // the construction method below is taken from https://github.com/soffes/HotKey/blob/c13662730cb5bc28de4a799854bbb018a90649bf/Sources/HotKey/HotKeysController.swift#L27
                    // and confirmed by applying the same method to `kEventParamDragRef` which is equal to `drag` in C
                    // and converted to `1685217639` by rust-bindgen.
                    for c in "htrs".chars() {
                        res = (res << 8) + c as u32;
                    }
                    res
                },
            };

            let ptr = unsafe {
                let mut hotkey_ref: EventHotKeyRef = std::mem::zeroed();
                let result = RegisterEventHotKey(
                    scan_code,
                    mods,
                    hotkey_id,
                    GetApplicationEventTarget(),
                    0,
                    &mut hotkey_ref,
                );

                if result != noErr as _ {
                    return Err(crate::Error::FailedToRegister(format!(
                        "Unable to register hotkey: {}",
                        hotkey.key
                    )));
                }

                hotkey_ref
            };

            self.hotkeys
                .borrow_mut()
                .insert(hotkey.id(), HotKeyWrapper { ptr, hotkey });
            Ok(())
        } else {
            Err(crate::Error::FailedToRegister(format!(
                "Unable to register accelerator (unknown scancode for this key: {}).",
                hotkey.key
            )))
        }
    }

    pub fn unregister(&self, hotkey: HotKey) -> crate::Result<()> {
        if let Some(hotkeywrapper) = self.hotkeys.borrow_mut().remove(&hotkey.id()) {
            unsafe { self.unregister_hotkey_ptr(hotkeywrapper.ptr, hotkey) }?;
        }

        Ok(())
    }

    pub fn unregister_all(&self) -> crate::Result<()> {
        let hotkeys = self.hotkeys.borrow().clone();
        for (_, hotkeywrapper) in hotkeys {
            self.unregister(hotkeywrapper.hotkey)?;
        }
        Ok(())
    }

    unsafe fn unregister_hotkey_ptr(
        &self,
        ptr: EventHotKeyRef,
        hotkey: HotKey,
    ) -> crate::Result<()> {
        if UnregisterEventHotKey(ptr) != noErr as _ {
            return Err(crate::Error::FailedToUnRegister(hotkey));
        }

        Ok(())
    }
}

impl Drop for GlobalHotKeyManager {
    fn drop(&mut self) {
        let _ = self.unregister_all();
        unsafe {
            RemoveEventHandler(self.event_handler_ptr);
        }
    }
}

unsafe extern "C" fn hotkey_handler(
    _next_handler: EventHandlerCallRef,
    event: EventRef,
    _user_data: *mut c_void,
) -> OSStatus {
    let mut event_hotkey: EventHotKeyID = std::mem::zeroed();

    let result = GetEventParameter(
        event,
        kEventParamDirectObject,
        typeEventHotKeyID,
        std::ptr::null_mut(),
        std::mem::size_of::<EventHotKeyID>() as _,
        std::ptr::null_mut(),
        &mut event_hotkey as *mut _ as *mut _,
    );

    if result == noErr as _ {
        let _ = GlobalHotKeyEvent::send(GlobalHotKeyEvent {
            id: event_hotkey.id,
        });
    }

    noErr as _
}

#[derive(Clone, Copy, Debug)]
struct HotKeyWrapper {
    ptr: EventHotKeyRef,
    hotkey: HotKey,
}

pub fn key_to_scancode(code: Code) -> Option<u32> {
    match code {
        Code::KeyA => Some(0x00),
        Code::KeyS => Some(0x01),
        Code::KeyD => Some(0x02),
        Code::KeyF => Some(0x03),
        Code::KeyH => Some(0x04),
        Code::KeyG => Some(0x05),
        Code::KeyZ => Some(0x06),
        Code::KeyX => Some(0x07),
        Code::KeyC => Some(0x08),
        Code::KeyV => Some(0x09),
        Code::KeyB => Some(0x0b),
        Code::KeyQ => Some(0x0c),
        Code::KeyW => Some(0x0d),
        Code::KeyE => Some(0x0e),
        Code::KeyR => Some(0x0f),
        Code::KeyY => Some(0x10),
        Code::KeyT => Some(0x11),
        Code::Digit1 => Some(0x12),
        Code::Digit2 => Some(0x13),
        Code::Digit3 => Some(0x14),
        Code::Digit4 => Some(0x15),
        Code::Digit6 => Some(0x16),
        Code::Digit5 => Some(0x17),
        Code::Equal => Some(0x18),
        Code::Digit9 => Some(0x19),
        Code::Digit7 => Some(0x1a),
        Code::Minus => Some(0x1b),
        Code::Digit8 => Some(0x1c),
        Code::Digit0 => Some(0x1d),
        Code::BracketRight => Some(0x1e),
        Code::KeyO => Some(0x1f),
        Code::KeyU => Some(0x20),
        Code::BracketLeft => Some(0x21),
        Code::KeyI => Some(0x22),
        Code::KeyP => Some(0x23),
        Code::Enter => Some(0x24),
        Code::KeyL => Some(0x25),
        Code::KeyJ => Some(0x26),
        Code::Quote => Some(0x27),
        Code::KeyK => Some(0x28),
        Code::Semicolon => Some(0x29),
        Code::Backslash => Some(0x2a),
        Code::Comma => Some(0x2b),
        Code::Slash => Some(0x2c),
        Code::KeyN => Some(0x2d),
        Code::KeyM => Some(0x2e),
        Code::Period => Some(0x2f),
        Code::Tab => Some(0x30),
        Code::Space => Some(0x31),
        Code::Backquote => Some(0x32),
        Code::Backspace => Some(0x33),
        Code::Escape => Some(0x35),
        Code::F17 => Some(0x40),
        Code::NumpadDecimal => Some(0x41),
        Code::NumpadMultiply => Some(0x43),
        Code::NumpadAdd => Some(0x45),
        Code::NumLock => Some(0x47),
        Code::AudioVolumeUp => Some(0x48),
        Code::AudioVolumeDown => Some(0x49),
        Code::AudioVolumeMute => Some(0x4a),
        Code::NumpadDivide => Some(0x4b),
        Code::NumpadEnter => Some(0x4c),
        Code::NumpadSubtract => Some(0x4e),
        Code::F18 => Some(0x4f),
        Code::F19 => Some(0x50),
        Code::NumpadEqual => Some(0x51),
        Code::Numpad0 => Some(0x52),
        Code::Numpad1 => Some(0x53),
        Code::Numpad2 => Some(0x54),
        Code::Numpad3 => Some(0x55),
        Code::Numpad4 => Some(0x56),
        Code::Numpad5 => Some(0x57),
        Code::Numpad6 => Some(0x58),
        Code::Numpad7 => Some(0x59),
        Code::F20 => Some(0x5a),
        Code::Numpad8 => Some(0x5b),
        Code::Numpad9 => Some(0x5c),
        Code::F5 => Some(0x60),
        Code::F6 => Some(0x61),
        Code::F7 => Some(0x62),
        Code::F3 => Some(0x63),
        Code::F8 => Some(0x64),
        Code::F9 => Some(0x65),
        Code::F11 => Some(0x67),
        Code::F13 => Some(0x69),
        Code::F16 => Some(0x6a),
        Code::F14 => Some(0x6b),
        Code::F10 => Some(0x6d),
        Code::F12 => Some(0x6f),
        Code::F15 => Some(0x71),
        Code::Insert => Some(0x72),
        Code::Home => Some(0x73),
        Code::PageUp => Some(0x74),
        Code::Delete => Some(0x75),
        Code::F4 => Some(0x76),
        Code::End => Some(0x77),
        Code::F2 => Some(0x78),
        Code::PageDown => Some(0x79),
        Code::F1 => Some(0x7a),
        Code::ArrowLeft => Some(0x7b),
        Code::ArrowRight => Some(0x7c),
        Code::ArrowDown => Some(0x7d),
        Code::ArrowUp => Some(0x7e),
        Code::CapsLock => Some(0x39),
        Code::PrintScreen => Some(0x46),
        _ => None,
    }
}
