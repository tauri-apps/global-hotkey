// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::str::FromStr;

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::Session;
use ashpd::AppID;
use crossbeam_channel::{unbounded, Receiver, Select, Sender};
use futures::{Stream, StreamExt};
use keyboard_types::{Code, Modifiers};

use crate::{
    hotkey::{DescribedHotKey, HotKey},
    Error, GlobalHotKeyEvent, HotKeyState,
};

use super::ThreadMessage;

enum GSEvent {
    Activated(ashpd::desktop::global_shortcuts::Activated),
    Deactivated(ashpd::desktop::global_shortcuts::Deactivated),
    /// The D-Bus signal stream ended (e.g. the portal restarted).
    StreamEnded,
}

struct GlobalShortcutsState<'a> {
    proxy: GlobalShortcuts<'a>,
    session: Session<'a, GlobalShortcuts<'a>>,
}

fn resolve_app_id() -> String {
    std::env::var("GLOBAL_HOTKEY_APP_ID")
        .or_else(|_| std::env::var("FLATPAK_ID"))
        .unwrap_or_else(|_| "com.global-hotkey.app".to_string())
}

impl GlobalShortcutsState<'_> {
    async fn new(app_id: &str, event_sender: Sender<GSEvent>) -> Result<Self, String> {
        match AppID::from_str(app_id) {
            Ok(app_id) => {
                if let Err(_e) = ashpd::register_host_app(app_id).await {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Failed to register app id: {:?}", _e);
                }
            }
            Err(_e) => {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to parse app id: {:?}", _e);
            }
        }

        let proxy = GlobalShortcuts::new()
            .await
            .map_err(|e| format!("Failed to start global shortcuts portal proxy: {e}"))?;

        let session = proxy
            .create_session()
            .await
            .map_err(|e| format!("Failed to start global shortcuts portal session: {e}"))?;

        let mut event_stream = Self::get_event_stream(&proxy).await?;

        tokio::spawn(async move {
            while let Some(ev) = event_stream.next().await {
                let _ = event_sender.send(ev);
            }
            let _ = event_sender.send(GSEvent::StreamEnded);
        });

        Ok(Self { proxy, session })
    }

    async fn get_event_stream(
        proxy: &GlobalShortcuts<'_>,
    ) -> Result<Box<dyn Stream<Item = GSEvent> + Unpin + Send>, String> {
        let activated: Box<dyn Stream<Item = GSEvent> + Unpin + Send> = Box::new(
            proxy
                .receive_activated()
                .await
                .map_err(|e| format!("Failed to receive portal activated stream: {e}"))?
                .map(GSEvent::Activated),
        );
        let deactivated: Box<dyn Stream<Item = GSEvent> + Unpin + Send> = Box::new(
            proxy
                .receive_deactivated()
                .await
                .map_err(|e| format!("Failed to receive portal deactivated stream: {e}"))?
                .map(GSEvent::Deactivated),
        );

        Ok(Box::new(futures::stream::select(activated, deactivated)))
    }
}

async fn rebind_all(
    gs_state: &mut GlobalShortcutsState<'_>,
    registered_hotkeys: &HashMap<u32, DescribedHotKey>,
) -> Result<(), Error> {
    // Close failure is non-fatal (e.g. the portal restarted and the session is
    // already gone); creating the new session below self-heals.
    if let Err(_e) = gs_state.session.close().await {
        #[cfg(feature = "tracing")]
        tracing::warn!("Failed to close old global shortcuts session: {_e}");
    }

    gs_state.session = gs_state.proxy.create_session().await.map_err(|e| {
        Error::FailedToRegister(format!(
            "Failed to start global shortcuts portal session: {e}"
        ))
    })?;

    let shortcuts: Vec<NewShortcut> = registered_hotkeys
        .iter()
        .map(|(id, dh)| {
            NewShortcut::new(
                id.to_string(),
                dh.description
                    .clone()
                    .unwrap_or_else(|| dh.hotkey.into_string()),
            )
            .preferred_trigger(hotkey_to_wayland_trigger(dh.hotkey).as_deref())
        })
        .collect();

    // Not handling error from BindShortcuts due to GNOME 48 bug (fixed in GNOME 49):
    // https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/issues/177
    let _ = gs_state
        .proxy
        .bind_shortcuts(&gs_state.session, &shortcuts, None)
        .await
        .map(|r| r.response());

    Ok(())
}

/// Outcome of the Wayland event loop attempt.
pub enum Outcome {
    /// The portal handled the whole session lifetime; nothing else to do.
    Handled,
    /// The GlobalShortcuts portal was unavailable at startup. The receiver is
    /// returned untouched so the caller can fall back to the X11 backend.
    Unavailable(Receiver<ThreadMessage>),
}

pub fn events_processor(thread_rx: Receiver<ThreadMessage>) -> Result<Outcome, String> {
    // Must use multi_thread runtime because the event loop uses crossbeam::Select::select()
    // which blocks the current thread. The tokio::spawn'd D-Bus event stream reader needs a
    // separate worker thread to make progress while Select blocks.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create tokio runtime: {e}"))?;
    rt.block_on(events_processor_async(thread_rx))
}

async fn events_processor_async(thread_rx: Receiver<ThreadMessage>) -> Result<Outcome, String> {
    let mut registered_hotkeys = HashMap::<u32, DescribedHotKey>::new();
    let mut hotkey_pressed = HashMap::<u32, bool>::new();

    let (gs_event_sender, gs_event_receiver) = unbounded();

    let app_id = resolve_app_id();

    // Probe the portal before entering the event loop. If the GlobalShortcuts
    // portal is missing (older compositors, no xdg-desktop-portal backend), hand
    // the receiver back so the caller can fall back to X11 instead of erroring.
    // Keep `gs_event_sender` alive: if it were moved and the stream task died,
    // the closed channel would make Select below busy-loop.
    let mut gs_state = match GlobalShortcutsState::new(&app_id, gs_event_sender.clone()).await {
        Ok(gs) => gs,
        Err(_e) => {
            #[cfg(feature = "tracing")]
            tracing::warn!("GlobalShortcuts portal unavailable, falling back to X11: {_e}");
            return Ok(Outcome::Unavailable(thread_rx));
        }
    };

    let mut select = Select::new();
    let thread_rx_idx = select.recv(&thread_rx);
    let gs_rx_idx = select.recv(&gs_event_receiver);

    loop {
        let selected_oper = select.select();
        match selected_oper.index() {
            i if i == thread_rx_idx => match selected_oper.recv(&thread_rx) {
                Ok(ThreadMessage::RegisterHotKey(dh, tx)) => {
                    let id = dh.hotkey.id();
                    let prev = registered_hotkeys.insert(id, dh);
                    let result = rebind_all(&mut gs_state, &registered_hotkeys).await;
                    // Roll back so a failed hotkey doesn't get silently bound
                    // by a later successful rebind.
                    if result.is_err() && prev.is_none() {
                        registered_hotkeys.remove(&id);
                    }
                    let _ = tx.send(result);
                }
                Ok(ThreadMessage::RegisterHotKeys(dhs, tx)) => {
                    let mut new_ids = Vec::new();
                    for dh in dhs {
                        let id = dh.hotkey.id();
                        if registered_hotkeys.insert(id, dh).is_none() {
                            new_ids.push(id);
                        }
                    }
                    let result = rebind_all(&mut gs_state, &registered_hotkeys).await;
                    if result.is_err() {
                        for id in &new_ids {
                            registered_hotkeys.remove(id);
                        }
                    }
                    let _ = tx.send(result);
                }
                Ok(ThreadMessage::UnRegisterHotKey(hotkey, tx)) => {
                    registered_hotkeys.remove(&hotkey.id());
                    hotkey_pressed.remove(&hotkey.id());
                    let result = rebind_all(&mut gs_state, &registered_hotkeys).await;
                    let _ = tx.send(result);
                }
                Ok(ThreadMessage::UnRegisterHotKeys(hotkeys, tx)) => {
                    for hotkey in &hotkeys {
                        registered_hotkeys.remove(&hotkey.id());
                        hotkey_pressed.remove(&hotkey.id());
                    }
                    let result = rebind_all(&mut gs_state, &registered_hotkeys).await;
                    let _ = tx.send(result);
                }
                Ok(ThreadMessage::TriggerDescription(hotkey, tx)) => {
                    let _ = tx.send(trigger_description(&gs_state, hotkey).await);
                }
                Ok(ThreadMessage::DropThread) => return Ok(Outcome::Handled),
                Err(_) => return Ok(Outcome::Handled),
            },
            i if i == gs_rx_idx => match selected_oper.recv(&gs_event_receiver) {
                Ok(GSEvent::Activated(activated)) => {
                    if let Some(id) = activated
                        .shortcut_id()
                        .parse::<u32>()
                        .ok()
                        .filter(|id| registered_hotkeys.contains_key(id))
                    {
                        let already_pressed = hotkey_pressed.get(&id).copied().unwrap_or(false);
                        if !already_pressed {
                            hotkey_pressed.insert(id, true);
                            GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                                id,
                                state: HotKeyState::Pressed,
                            });
                        }
                    }
                }
                Ok(GSEvent::Deactivated(deactivated)) => {
                    if let Some(id) = deactivated
                        .shortcut_id()
                        .parse::<u32>()
                        .ok()
                        .filter(|id| registered_hotkeys.contains_key(id))
                    {
                        hotkey_pressed.insert(id, false);
                        GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                            id,
                            state: HotKeyState::Released,
                        });
                    }
                }
                Ok(GSEvent::StreamEnded) => {
                    // The portal (or D-Bus connection) went away. Wait briefly to
                    // avoid a tight loop, then recreate the session and rebind so
                    // hotkeys keep working after a portal restart.
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    match GlobalShortcutsState::new(&app_id, gs_event_sender.clone()).await {
                        Ok(gs) => {
                            gs_state = gs;
                            if let Err(_e) = rebind_all(&mut gs_state, &registered_hotkeys).await {
                                #[cfg(feature = "tracing")]
                                tracing::warn!("Failed to rebind after portal restart: {_e}");
                            }
                        }
                        Err(_e) => {
                            #[cfg(feature = "tracing")]
                            tracing::warn!("Failed to reconnect to portal: {_e}");
                        }
                    }
                }
                Err(_) => {}
            },
            _ => unreachable!(),
        }
    }
}

/// Queries the portal for the trigger currently bound to this hotkey; the
/// user may have reassigned it in the system settings. Falls back to the
/// hotkey's own string representation if the portal reports nothing.
async fn trigger_description(
    gs_state: &GlobalShortcutsState<'_>,
    hotkey: HotKey,
) -> crate::Result<String> {
    let response = gs_state
        .proxy
        .list_shortcuts(&gs_state.session)
        .await
        .and_then(|r| r.response())
        .map_err(|e| {
            Error::OsError(std::io::Error::other(format!(
                "Failed to list global shortcuts from portal: {e}"
            )))
        })?;

    let id = hotkey.id().to_string();
    Ok(response
        .shortcuts()
        .iter()
        .find(|s| s.id() == id)
        .map(|s| s.trigger_description().to_string())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| hotkey.into_string()))
}

/// Keys whose keysym changes with SHIFT depending on the keyboard layout
/// (e.g. Shift+8 produces `asterisk` on US layouts but `(` on German ones).
/// The XDG shortcuts spec encodes triggers as modifiers + keysym, so a literal
/// trigger like "CTRL+SHIFT+8" can never fire; without access to the active
/// keymap the correct shifted keysym is unknowable here.
fn shift_changes_keysym(key: Code) -> bool {
    matches!(
        key,
        Code::Digit0
            | Code::Digit1
            | Code::Digit2
            | Code::Digit3
            | Code::Digit4
            | Code::Digit5
            | Code::Digit6
            | Code::Digit7
            | Code::Digit8
            | Code::Digit9
            | Code::Backquote
            | Code::Minus
            | Code::Equal
            | Code::BracketLeft
            | Code::BracketRight
            | Code::Backslash
            | Code::Semicolon
            | Code::Quote
            | Code::Comma
            | Code::Period
            | Code::Slash
    )
}

fn hotkey_to_wayland_trigger(hotkey: HotKey) -> Option<String> {
    // Omit the preferred trigger and let the compositor prompt the user for a
    // combination instead of requesting one that cannot fire.
    if hotkey.mods.contains(Modifiers::SHIFT) && shift_changes_keysym(hotkey.key) {
        return None;
    }

    let mut mods = String::new();

    if hotkey.mods.contains(Modifiers::CONTROL) {
        mods += "CTRL+";
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        mods += "SHIFT+";
    }
    if hotkey.mods.contains(Modifiers::ALT) {
        mods += "ALT+";
    }
    if hotkey.mods.intersects(Modifiers::SUPER | Modifiers::META) {
        mods += "LOGO+";
    }

    let keycode = match hotkey.key {
        Code::KeyA => "a",
        Code::KeyB => "b",
        Code::KeyC => "c",
        Code::KeyD => "d",
        Code::KeyE => "e",
        Code::KeyF => "f",
        Code::KeyG => "g",
        Code::KeyH => "h",
        Code::KeyI => "i",
        Code::KeyJ => "j",
        Code::KeyK => "k",
        Code::KeyL => "l",
        Code::KeyM => "m",
        Code::KeyN => "n",
        Code::KeyO => "o",
        Code::KeyP => "p",
        Code::KeyQ => "q",
        Code::KeyR => "r",
        Code::KeyS => "s",
        Code::KeyT => "t",
        Code::KeyU => "u",
        Code::KeyV => "v",
        Code::KeyW => "w",
        Code::KeyX => "x",
        Code::KeyY => "y",
        Code::KeyZ => "z",
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
        Code::ArrowDown => "Down",
        Code::ArrowLeft => "Left",
        Code::ArrowRight => "Right",
        Code::ArrowUp => "Up",
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
        Code::NumLock => "Num_Lock",
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
        Code::F13 => "F13",
        Code::F14 => "F14",
        Code::F15 => "F15",
        Code::F16 => "F16",
        Code::F17 => "F17",
        Code::F18 => "F18",
        Code::F19 => "F19",
        Code::F20 => "F20",
        Code::F21 => "F21",
        Code::F22 => "F22",
        Code::F23 => "F23",
        Code::F24 => "F24",
        Code::AudioVolumeDown => "XF86AudioLowerVolume",
        Code::AudioVolumeMute => "XF86AudioMute",
        Code::AudioVolumeUp => "XF86AudioRaiseVolume",
        Code::MediaPlay => "XF86AudioPlay",
        Code::MediaPlayPause => "XF86AudioPlay",
        Code::MediaPause => "XF86AudioPause",
        Code::MediaStop => "XF86AudioStop",
        Code::MediaTrackNext => "XF86AudioNext",
        Code::MediaTrackPrevious => "XF86AudioPrev",
        Code::MediaSelect => "XF86AudioMedia",
        Code::BrowserBack => "XF86Back",
        Code::BrowserFavorites => "XF86Favorites",
        Code::BrowserForward => "XF86Forward",
        Code::BrowserHome => "XF86HomePage",
        Code::BrowserRefresh => "XF86Refresh",
        Code::BrowserSearch => "XF86Search",
        Code::BrowserStop => "XF86Stop",
        Code::LaunchApp1 => "XF86Explorer",
        Code::LaunchApp2 => "XF86Calculator",
        Code::LaunchMail => "XF86Mail",
        Code::Eject => "XF86Eject",
        Code::WakeUp => "XF86WakeUp",
        Code::Pause => "Pause",
        _ => return None,
    };

    Some(mods + keycode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_basic_combos() {
        let hk = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyX);
        assert_eq!(
            hotkey_to_wayland_trigger(hk).as_deref(),
            Some("CTRL+SHIFT+x")
        );
        let hk = HotKey::new(Some(Modifiers::SUPER), Code::F13);
        assert_eq!(hotkey_to_wayland_trigger(hk).as_deref(), Some("LOGO+F13"));
        let hk = HotKey::new(None, Code::BrowserFavorites);
        assert_eq!(
            hotkey_to_wayland_trigger(hk).as_deref(),
            Some("XF86Favorites")
        );
    }

    #[test]
    fn trigger_omitted_for_layout_dependent_shift_combos() {
        // Shift+8 produces a layout-dependent keysym; a literal trigger would
        // never fire, so no preferred trigger must be emitted.
        let hk = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Digit8);
        assert_eq!(hotkey_to_wayland_trigger(hk), None);
        // Without SHIFT the digit keysym is stable.
        let hk = HotKey::new(Some(Modifiers::CONTROL), Code::Digit8);
        assert_eq!(hotkey_to_wayland_trigger(hk).as_deref(), Some("CTRL+8"));
    }
}
