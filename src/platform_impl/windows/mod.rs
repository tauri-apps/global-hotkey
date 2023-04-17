// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::ptr;

use keyboard_types::{Code, Modifiers};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            RegisterHotKey, UnregisterHotKey, VkKeyScanW, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
            MOD_SHIFT, MOD_WIN, VIRTUAL_KEY, VK_APPS, VK_BACK, VK_BROWSER_BACK,
            VK_BROWSER_FAVORITES, VK_BROWSER_FORWARD, VK_BROWSER_HOME, VK_BROWSER_REFRESH,
            VK_BROWSER_SEARCH, VK_BROWSER_STOP, VK_CAPITAL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE,
            VK_F1, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14, VK_F15, VK_F16, VK_F17, VK_F18, VK_F19,
            VK_F2, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7,
            VK_F8, VK_F9, VK_HELP, VK_HOME, VK_INSERT, VK_KANA, VK_LAUNCH_MAIL, VK_LEFT,
            VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_NEXT,
            VK_NONCONVERT, VK_NUMLOCK, VK_OEM_COMMA, VK_OEM_MINUS, VK_OEM_PERIOD, VK_PAUSE,
            VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SCROLL, VK_SNAPSHOT, VK_SPACE, VK_TAB, VK_UP,
            VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
        },
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, RegisterClassW, CW_USEDEFAULT, HMENU, WM_HOTKEY,
            WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
            WS_OVERLAPPED,
        },
    },
};

use crate::{hotkey::HotKey, GlobalHotKeyEvent};

const GLOBAL_HOTKEY_SUBCLASS_ID: usize = 6001;

pub struct GlobalHotKeyManager {
    hwnd: isize,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        let class_name = encode_wide("tray_icon_app");
        unsafe {
            let hinstance = get_instance_handle();

            unsafe extern "system" fn call_default_window_proc(
                hwnd: HWND,
                msg: u32,
                wparam: WPARAM,
                lparam: LPARAM,
            ) -> LRESULT {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(call_default_window_proc),
                lpszClassName: class_name.as_ptr(),
                hInstance: hinstance,
                ..std::mem::zeroed()
            };

            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED |
                // WS_EX_TOOLWINDOW prevents this window from ever showing up in the taskbar, which
                // we want to avoid. If you remove this style, this window won't show up in the
                // taskbar *initially*, but it can show up at some later point. This can sometimes
                // happen on its own after several hours have passed, although this has proven
                // difficult to reproduce. Alternatively, it can be manually triggered by killing
                // `explorer.exe` and then starting the process back up.
                // It is unclear why the bug is triggered by waiting for several hours.
                WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                ptr::null(),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                0,
                CW_USEDEFAULT,
                0,
                HWND::default(),
                HMENU::default(),
                hinstance,
                std::ptr::null_mut(),
            );
            if hwnd == 0 {
                return Err(crate::Error::OsError(std::io::Error::last_os_error()));
            }

            SetWindowSubclass(
                hwnd,
                Some(global_hotkey_subclass_proc),
                GLOBAL_HOTKEY_SUBCLASS_ID,
                0,
            );

            Ok(Self { hwnd })
        }
    }

    pub fn register(&self, hotkey: HotKey) -> crate::Result<()> {
        let mut mods = MOD_NOREPEAT;
        if hotkey.mods.contains(Modifiers::SHIFT) {
            mods |= MOD_SHIFT;
        }
        if hotkey.mods.intersects(Modifiers::SUPER | Modifiers::META) {
            mods |= MOD_WIN;
        }
        if hotkey.mods.contains(Modifiers::ALT) {
            mods |= MOD_ALT;
        }
        if hotkey.mods.contains(Modifiers::CONTROL) {
            mods |= MOD_CONTROL;
        }

        // get key scan code
        match key_to_vk(&hotkey.key) {
            Some(vk_code) => {
                let result =
                    unsafe { RegisterHotKey(self.hwnd, hotkey.id() as _, mods, vk_code as _) };
                if result == 0 {
                    return Err(crate::Error::AlreadyRegistered(hotkey));
                }
            }
            _ => {
                return Err(crate::Error::FailedToRegister(format!(
                    "Unable to register hotkey (unknown VKCode for this key: {}).",
                    hotkey.key
                )))
            }
        }

        Ok(())
    }

    pub fn unregister(&self, hotkey: HotKey) -> crate::Result<()> {
        let result = unsafe { UnregisterHotKey(self.hwnd, hotkey.id() as _) };
        if result == 0 {
            return Err(crate::Error::OsError(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
unsafe extern "system" fn global_hotkey_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _subclass_input_ptr: usize,
) -> LRESULT {
    if msg == WM_HOTKEY {
        GlobalHotKeyEvent::send(GlobalHotKeyEvent { id: wparam as _ });
    }

    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub fn encode_wide<S: AsRef<std::ffi::OsStr>>(string: S) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

pub fn get_instance_handle() -> windows_sys::Win32::Foundation::HMODULE {
    // Gets the instance handle by taking the address of the
    // pseudo-variable created by the microsoft linker:
    // https://devblogs.microsoft.com/oldnewthing/20041025-00/?p=37483

    // This is preferred over GetModuleHandle(NULL) because it also works in DLLs:
    // https://stackoverflow.com/questions/21718027/getmodulehandlenull-vs-hinstance

    extern "C" {
        static __ImageBase: windows_sys::Win32::System::SystemServices::IMAGE_DOS_HEADER;
    }

    unsafe { &__ImageBase as *const _ as _ }
}

// used to build accelerators table from Key
fn key_to_vk(key: &Code) -> Option<VIRTUAL_KEY> {
    Some(match key {
        Code::KeyA => unsafe { VkKeyScanW('a' as u16) as u16 },
        Code::KeyB => unsafe { VkKeyScanW('b' as u16) as u16 },
        Code::KeyC => unsafe { VkKeyScanW('c' as u16) as u16 },
        Code::KeyD => unsafe { VkKeyScanW('d' as u16) as u16 },
        Code::KeyE => unsafe { VkKeyScanW('e' as u16) as u16 },
        Code::KeyF => unsafe { VkKeyScanW('f' as u16) as u16 },
        Code::KeyG => unsafe { VkKeyScanW('g' as u16) as u16 },
        Code::KeyH => unsafe { VkKeyScanW('h' as u16) as u16 },
        Code::KeyI => unsafe { VkKeyScanW('i' as u16) as u16 },
        Code::KeyJ => unsafe { VkKeyScanW('j' as u16) as u16 },
        Code::KeyK => unsafe { VkKeyScanW('k' as u16) as u16 },
        Code::KeyL => unsafe { VkKeyScanW('l' as u16) as u16 },
        Code::KeyM => unsafe { VkKeyScanW('m' as u16) as u16 },
        Code::KeyN => unsafe { VkKeyScanW('n' as u16) as u16 },
        Code::KeyO => unsafe { VkKeyScanW('o' as u16) as u16 },
        Code::KeyP => unsafe { VkKeyScanW('p' as u16) as u16 },
        Code::KeyQ => unsafe { VkKeyScanW('q' as u16) as u16 },
        Code::KeyR => unsafe { VkKeyScanW('r' as u16) as u16 },
        Code::KeyS => unsafe { VkKeyScanW('s' as u16) as u16 },
        Code::KeyT => unsafe { VkKeyScanW('t' as u16) as u16 },
        Code::KeyU => unsafe { VkKeyScanW('u' as u16) as u16 },
        Code::KeyV => unsafe { VkKeyScanW('v' as u16) as u16 },
        Code::KeyW => unsafe { VkKeyScanW('w' as u16) as u16 },
        Code::KeyX => unsafe { VkKeyScanW('x' as u16) as u16 },
        Code::KeyY => unsafe { VkKeyScanW('y' as u16) as u16 },
        Code::KeyZ => unsafe { VkKeyScanW('z' as u16) as u16 },
        Code::Digit0 => unsafe { VkKeyScanW('0' as u16) as u16 },
        Code::Digit1 => unsafe { VkKeyScanW('1' as u16) as u16 },
        Code::Digit2 => unsafe { VkKeyScanW('2' as u16) as u16 },
        Code::Digit3 => unsafe { VkKeyScanW('3' as u16) as u16 },
        Code::Digit4 => unsafe { VkKeyScanW('4' as u16) as u16 },
        Code::Digit5 => unsafe { VkKeyScanW('5' as u16) as u16 },
        Code::Digit6 => unsafe { VkKeyScanW('6' as u16) as u16 },
        Code::Digit7 => unsafe { VkKeyScanW('7' as u16) as u16 },
        Code::Digit8 => unsafe { VkKeyScanW('8' as u16) as u16 },
        Code::Digit9 => unsafe { VkKeyScanW('9' as u16) as u16 },
        Code::Comma => VK_OEM_COMMA,
        Code::Minus => VK_OEM_MINUS,
        Code::Period => VK_OEM_PERIOD,
        Code::Equal => unsafe { VkKeyScanW('=' as u16) as u16 },
        Code::Semicolon => unsafe { VkKeyScanW(';' as u16) as u16 },
        Code::Slash => unsafe { VkKeyScanW('/' as u16) as u16 },
        Code::Backslash => unsafe { VkKeyScanW('\\' as u16) as u16 },
        Code::Quote => unsafe { VkKeyScanW('\'' as u16) as u16 },
        Code::Backquote => unsafe { VkKeyScanW('`' as u16) as u16 },
        Code::BracketLeft => unsafe { VkKeyScanW('[' as u16) as u16 },
        Code::BracketRight => unsafe { VkKeyScanW(']' as u16) as u16 },
        Code::Backspace => VK_BACK,
        Code::Tab => VK_TAB,
        Code::Space => VK_SPACE,
        Code::Enter => VK_RETURN,
        Code::Pause => VK_PAUSE,
        Code::CapsLock => VK_CAPITAL,
        Code::KanaMode => VK_KANA,
        Code::Escape => VK_ESCAPE,
        Code::NonConvert => VK_NONCONVERT,
        Code::PageUp => VK_PRIOR,
        Code::PageDown => VK_NEXT,
        Code::End => VK_END,
        Code::Home => VK_HOME,
        Code::ArrowLeft => VK_LEFT,
        Code::ArrowUp => VK_UP,
        Code::ArrowRight => VK_RIGHT,
        Code::ArrowDown => VK_DOWN,
        Code::PrintScreen => VK_SNAPSHOT,
        Code::Insert => VK_INSERT,
        Code::Delete => VK_DELETE,
        Code::Help => VK_HELP,
        Code::ContextMenu => VK_APPS,
        Code::F1 => VK_F1,
        Code::F2 => VK_F2,
        Code::F3 => VK_F3,
        Code::F4 => VK_F4,
        Code::F5 => VK_F5,
        Code::F6 => VK_F6,
        Code::F7 => VK_F7,
        Code::F8 => VK_F8,
        Code::F9 => VK_F9,
        Code::F10 => VK_F10,
        Code::F11 => VK_F11,
        Code::F12 => VK_F12,
        Code::F13 => VK_F13,
        Code::F14 => VK_F14,
        Code::F15 => VK_F15,
        Code::F16 => VK_F16,
        Code::F17 => VK_F17,
        Code::F18 => VK_F18,
        Code::F19 => VK_F19,
        Code::F20 => VK_F20,
        Code::F21 => VK_F21,
        Code::F22 => VK_F22,
        Code::F23 => VK_F23,
        Code::F24 => VK_F24,
        Code::NumLock => VK_NUMLOCK,
        Code::ScrollLock => VK_SCROLL,
        Code::BrowserBack => VK_BROWSER_BACK,
        Code::BrowserForward => VK_BROWSER_FORWARD,
        Code::BrowserRefresh => VK_BROWSER_REFRESH,
        Code::BrowserStop => VK_BROWSER_STOP,
        Code::BrowserSearch => VK_BROWSER_SEARCH,
        Code::BrowserFavorites => VK_BROWSER_FAVORITES,
        Code::BrowserHome => VK_BROWSER_HOME,
        Code::AudioVolumeMute => VK_VOLUME_MUTE,
        Code::AudioVolumeDown => VK_VOLUME_DOWN,
        Code::AudioVolumeUp => VK_VOLUME_UP,
        Code::MediaTrackNext => VK_MEDIA_NEXT_TRACK,
        Code::MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
        Code::MediaStop => VK_MEDIA_STOP,
        Code::MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
        Code::LaunchMail => VK_LAUNCH_MAIL,
        Code::Convert => VK_INSERT,
        _ => return None,
    })
}
