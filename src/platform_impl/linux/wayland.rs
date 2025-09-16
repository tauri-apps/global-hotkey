// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{collections::HashMap, num::ParseIntError, str::FromStr};

use ashpd::{
    desktop::{
        global_shortcuts::{
            Activated, Deactivated, GlobalShortcuts, NewShortcut, Shortcut, ShortcutsChanged,
        },
        Session,
    },
    AppID,
};
use crossbeam_channel::{bounded, unbounded, Receiver, Select, Sender};
use futures::{stream::select_all, Stream, StreamExt};
use itertools::Itertools;
use keyboard_types::{Code, Modifiers};
use once_cell::sync::Lazy;

use crate::{
    hotkey::HotKey,
    platform_impl::platform::ThreadMessage,
    wayland::{WlHotKeyAction, WlNewHotKeyAction},
    Error, GlobalHotKeyEvent, HotKeyState,
};

enum GSEvent {
    Activated(Activated),
    Deactivated(Deactivated),
    Changed(ShortcutsChanged),
}

struct GlobalShortcutsState<'a> {
    proxy: GlobalShortcuts<'a>,
    session: Session<'a, GlobalShortcuts<'a>>,
}

impl GlobalShortcutsState<'_> {
    pub async fn new(event_sender: Sender<GSEvent>) -> Result<Self, String> {
        let proxy = GlobalShortcuts::new()
            .await
            .map_err(|e| format!("Failed to start global shortcuts portal proxy: {e}"))?;

        let session = proxy
            .create_session()
            .await
            .map_err(|e| format!("Failed to start global shortcuts portal session: {e}"))?;

        // combining the activated, deactivated, and shortcuts changed events into one stream
        let mut gs_event_stream = Self::get_event_stream(&proxy).await?;

        // listening for global shortcuts events in a separate thread
        tokio::spawn(async move {
            while let Some(ev) = gs_event_stream.next().await {
                let _ = event_sender.send(ev);
            }
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
                .map_err(|e| {
                    format!("Failed to receive global shortcuts portal activated stream: {e}")
                })?
                .map(GSEvent::Activated),
        );
        let deactivated = Box::new(
            proxy
                .receive_deactivated()
                .await
                .map_err(|e| {
                    format!("Failed to receive global shortcuts portal deactivated stream: {e}")
                })?
                .map(GSEvent::Deactivated),
        );
        let changed = Box::new(
            proxy
                .receive_shortcuts_changed()
                .await
                .map_err(|e| {
                    format!(
                        "Failed to receive global shortcuts portal shortcuts changed stream: {e}"
                    )
                })?
                .map(GSEvent::Changed),
        );

        Ok(Box::new(select_all([activated, deactivated, changed])))
    }
}

#[tokio::main]
pub async fn events_processor(thread_rx: Receiver<ThreadMessage>) -> Result<(), String> {
    let mut registered_hotkeys = Vec::<WlHotKeyAction>::new();
    let mut hotkey_states = HashMap::<u32, bool>::new();

    let (gs_event_sender, gs_event_receiver) = unbounded();
    let mut gs_state: Option<GlobalShortcutsState> = None;

    let mut select = Select::new();
    let thread_rx_idx = select.recv(&thread_rx);
    let gs_rx_idx = select.recv(&gs_event_receiver);
    loop {
        let selected_oper = select.select();
        match selected_oper.index() {
            i if i == thread_rx_idx => match selected_oper.recv(&thread_rx) {
                Ok(ThreadMessage::WlRegisterHotKeys(hotkeys, app_id, tx)) => {
                    if let Some(gs) = &mut gs_state {
                        let _ = tx
                            .send(reregister_hotkeys(gs, &mut registered_hotkeys, &hotkeys).await);
                    } else {
                        let _ = match init_global_shortcuts_with_app_id(
                            app_id,
                            gs_event_sender.clone(),
                        )
                        .await
                        {
                            Ok(mut new_gs) => {
                                let res = tx.send(
                                    reregister_hotkeys(
                                        &mut new_gs,
                                        &mut registered_hotkeys,
                                        &hotkeys,
                                    )
                                    .await,
                                );
                                gs_state = Some(new_gs);
                                res
                            }
                            Err(e) => tx.send(Err(Error::FailedToRegister(e))),
                        };
                    }
                }
                Ok(ThreadMessage::WlUnRegisterHotKeys(ids)) => {
                    registered_hotkeys.retain(|rh| !ids.contains(&rh.id()))
                }
                Ok(ThreadMessage::WlGetHotKeys(tx)) => {
                    let _ = tx.send(registered_hotkeys.clone().into());
                }
                Ok(ThreadMessage::DropThread) => return Ok(()),
                _ => {}
            },
            i if i == gs_rx_idx => {
                match selected_oper.recv(&gs_event_receiver) {
                    Ok(GSEvent::Activated(activated)) => {
                        // only send event if (1) shortcut id can be parsed as u32 and (2) if the
                        // shortcut has been registered
                        if let Some(id) = activated
                            .shortcut_id()
                            .parse::<u32>()
                            .ok()
                            .filter(|id| registered_hotkeys.iter().any(|rh| rh.id() == *id))
                        {
                            // only send event if not already pressed
                            if hotkey_states.get(&id).filter(|pressed| !*pressed).is_some() {
                                // update hotkey state before sending event
                                hotkey_states.insert(id, true);

                                GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                                    id,
                                    state: HotKeyState::Pressed,
                                });
                            }
                        }
                    }
                    Ok(GSEvent::Deactivated(deactivated)) => {
                        // only send event if (1) shortcut id can be parsed as u32 and (2) if the
                        // shortcut has been registered
                        if let Some(id) = deactivated
                            .shortcut_id()
                            .parse::<u32>()
                            .ok()
                            .filter(|id| registered_hotkeys.iter().any(|rh| rh.id() == *id))
                        {
                            // update hotkey state before sending event
                            hotkey_states.insert(id, false);

                            GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                                id,
                                state: HotKeyState::Released,
                            });
                        }
                    }
                    Ok(GSEvent::Changed(shortcuts_changed)) => {
                        let mut change = false;
                        for new in shortcuts_changed.shortcuts() {
                            if let Some(hk) = registered_hotkeys
                                .iter_mut()
                                .find(|rh| rh.id().to_string() == new.id())
                            {
                                if let Ok(new_hk) = new.clone().try_into() {
                                    *hk = new_hk;
                                    change = true;
                                }
                            }
                        }

                        if change {
                            let _ = WL_HOTKEYS_CHANGED_CHANNEL.0.send(WlHotKeysChangedEvent);
                        }
                    }
                    Err(_) => {}
                }
            }
            _ => unreachable!(),
        }
    }
}

async fn init_global_shortcuts_with_app_id<'a>(
    app_id: impl Into<String>,
    event_sender: Sender<GSEvent>,
) -> Result<GlobalShortcutsState<'a>, String> {
    let app_id =
        AppID::from_str(&app_id.into()).map_err(|e| format!("Failed to parse app id: {e}"))?;

    ashpd::register_host_app(app_id)
        .await
        .map_err(|e| format!("Failed to register app id: {e}"))?;

    GlobalShortcutsState::new(event_sender).await
}

async fn reregister_hotkeys<'a>(
    gs_state: &mut GlobalShortcutsState<'_>,
    registered_hotkeys: &mut Vec<WlHotKeyAction>,
    new_hotkeys: &[WlNewHotKeyAction],
) -> Result<(), Error> {
    gs_state.session.close().await.map_err(|e| {
        Error::FailedToRegister(format!("Failed to close old global shortcuts session: {e}"))
    })?;

    gs_state.session = gs_state.proxy.create_session().await.map_err(|e| {
        Error::FailedToRegister(format!(
            "Failed to start global shortcuts portal session: {e}"
        ))
    })?;

    // reregister all hotkeys in registered_hotkeys, plus everything in new_hotkeys that hasn't
    // already been registered
    let hotkeys_to_register = registered_hotkeys
        .iter()
        .cloned()
        .map(Into::into)
        .chain(
            new_hotkeys
                .iter()
                .unique_by(|nh| nh.id())
                .filter(|&nh| !registered_hotkeys.iter().any(|rh| rh.id() == nh.id()))
                .cloned()
                .map(Into::into),
        )
        .collect::<Vec<NewShortcut>>();

    // not handling error from BindShortcuts due to GNOME 48 bug (fixed in GNOME 49):
    // https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/issues/177
    let _ = gs_state
        .proxy
        .bind_shortcuts(&gs_state.session, &hotkeys_to_register, None)
        .await
        .map(|r| r.response());

    // update registered_shortcuts array
    if let Ok(ls) = gs_state
        .proxy
        .list_shortcuts(&gs_state.session)
        .await
        .and_then(|r| r.response())
    {
        registered_hotkeys.extend(
            ls.shortcuts()
                .iter()
                .filter(|sh| new_hotkeys.iter().any(|nh| nh.id().to_string() == sh.id()))
                .filter_map(|sh| sh.clone().try_into().ok()),
        )
    }

    Ok(())
}

pub struct WlHotKeysChangedEvent;
static WL_HOTKEYS_CHANGED_CHANNEL: Lazy<(
    Sender<WlHotKeysChangedEvent>,
    Receiver<WlHotKeysChangedEvent>,
)> = Lazy::new(|| bounded(1));

pub(crate) fn wl_hotkeys_changed_receiver() -> Receiver<WlHotKeysChangedEvent> {
    WL_HOTKEYS_CHANGED_CHANNEL.1.clone()
}

impl TryFrom<Shortcut> for WlHotKeyAction {
    type Error = ParseIntError;

    fn try_from(value: Shortcut) -> Result<Self, Self::Error> {
        let id = value.id().parse::<u32>()?;

        Ok(Self {
            id,
            action_description: value.description().into(),
            hotkey_description: value.trigger_description().into(),
        })
    }
}

impl From<WlHotKeyAction> for NewShortcut {
    fn from(wl_hotkey: WlHotKeyAction) -> Self {
        NewShortcut::new(wl_hotkey.id().to_string(), wl_hotkey.action_description())
    }
}

impl From<WlNewHotKeyAction> for NewShortcut {
    fn from(wl_hotkey: WlNewHotKeyAction) -> Self {
        NewShortcut::new(wl_hotkey.id().to_string(), wl_hotkey.description()).preferred_trigger(
            wl_hotkey
                .preferred_hotkey()
                .and_then(hotkey_to_wayland_trigger)
                .as_deref(),
        )
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
    if hotkey.mods.contains(Modifiers::SUPER) {
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

#[allow(unused)]
mod tests {
    use keyboard_types::Modifiers;

    use super::*;

    #[test]
    fn hotkey_to_wl_trigger_test() {
        let modifiers = Modifiers::SHIFT | Modifiers::META;
        let trigger_desc = hotkey_to_wayland_trigger(HotKey::new(Some(modifiers), Code::KeyD));
        assert_eq!(trigger_desc.as_deref(), Some("SHIFT+LOGO+D"));

        let modifiers = Modifiers::SHIFT | Modifiers::META | Modifiers::CONTROL | Modifiers::ALT;
        let trigger_desc = hotkey_to_wayland_trigger(HotKey::new(Some(modifiers), Code::Backslash));
        assert_eq!(
            trigger_desc.as_deref(),
            Some("CTRL+SHIFT+ALT+LOGO+backslash")
        )
    }
}
