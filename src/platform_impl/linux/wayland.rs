use std::{collections::HashMap, num::ParseIntError};

use ashpd::desktop::{
    global_shortcuts::{
        Activated, Deactivated, GlobalShortcuts, NewShortcut, Shortcut, ShortcutsChanged,
    },
    Session,
};
use crossbeam_channel::{bounded, Receiver, Select, Sender};
use futures::{stream::select_all, Stream, StreamExt};
use once_cell::sync::Lazy;

use crate::{
    error::Error,
    platform_impl::platform::ThreadMessage,
    wayland::{WlHotKey, WlNewHotKey},
    GlobalHotKeyEvent, HotKeyState,
};

enum GSEvent {
    Activated(Activated),
    Deactivated(Deactivated),
    Changed(ShortcutsChanged),
}

#[tokio::main]
pub async fn events_processor(thread_rx: Receiver<ThreadMessage>) -> Result<(), String> {
    let proxy = GlobalShortcuts::new()
        .await
        .map_err(|e| format!("Failed to start global shortcuts portal proxy: {e}"))?;
    let mut session = proxy
        .create_session()
        .await
        .map_err(|e| format!("Failed to start global shortcuts portal session: {e}"))?;

    let mut registered_hotkeys = Vec::<WlHotKey>::new();
    let mut hotkey_states = HashMap::<u32, bool>::new();

    let mut gs_event_stream: Box<dyn Stream<Item = GSEvent> + Unpin + Send> = {
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

        Box::new(select_all([activated, deactivated, changed]))
    };

    let (tx, gs_rx) = crossbeam_channel::unbounded();
    tokio::spawn(async move {
        while let Some(ev) = gs_event_stream.next().await {
            let _ = tx.send(ev);
        }
    });

    let mut select = Select::new();
    let thread_rx_idx = select.recv(&thread_rx);
    let gs_rx_idx = select.recv(&gs_rx);
    loop {
        let selected_oper = select.select();
        match selected_oper.index() {
            i if i == thread_rx_idx => match selected_oper.recv(&thread_rx) {
                Ok(ThreadMessage::WlRegisterHotKeys(hotkeys, tx)) => {
                    let _ = tx.send(
                        reregister_hotkeys(&proxy, &mut session, &mut registered_hotkeys, &hotkeys)
                            .await,
                    );
                }
                Ok(ThreadMessage::WlGetHotKeys(tx)) => {
                    let _ = tx.send(registered_hotkeys.clone().into());
                }
                Ok(ThreadMessage::DropThread) => return Ok(()),
                _ => {}
            },
            i if i == gs_rx_idx => {
                match selected_oper.recv(&gs_rx) {
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

async fn reregister_hotkeys<'a>(
    proxy: &GlobalShortcuts<'a>,
    session: &mut Session<'a, GlobalShortcuts<'a>>,
    registered_hotkeys: &mut Vec<WlHotKey>,
    new_hotkeys: &[WlNewHotKey],
) -> Result<(), Error> {
    session.close().await.map_err(|e| {
        Error::FailedToRegister(format!("Failed to close old global shortcuts session: {e}"))
    })?;

    *session = proxy.create_session().await.map_err(|e| {
        Error::FailedToRegister(format!(
            "Failed to start global shortcuts portal session: {e}"
        ))
    })?;

    // reregister all hotkeys in registered_hotkeys, plus everything in new_hotkeys that hasn't
    // already been registered
    let hotkeys_to_register = new_hotkeys
        .iter()
        .filter(|&nh| !registered_hotkeys.iter().any(|rh| rh.id() == nh.id()))
        .cloned()
        .map(Into::into)
        .chain(registered_hotkeys.iter().cloned().map(Into::into))
        .collect::<Vec<NewShortcut>>();

    // not handling error from BindShortcuts due to GNOME 48 bug (fixed in GNOME 49):
    // https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/issues/177
    let _ = proxy
        .bind_shortcuts(session, &hotkeys_to_register, None)
        .await
        .map(|r| r.response());

    // update registered_shortcuts array
    if let Ok(ls) = proxy
        .list_shortcuts(session)
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

impl TryFrom<Shortcut> for WlHotKey {
    type Error = ParseIntError;

    fn try_from(value: Shortcut) -> Result<Self, Self::Error> {
        let id = value.id().parse::<u32>()?;

        Ok(Self {
            id,
            description: value.description().into(),
            trigger_description: value.trigger_description().into(),
        })
    }
}

impl From<WlHotKey> for NewShortcut {
    fn from(wl_hotkey: WlHotKey) -> Self {
        NewShortcut::new(wl_hotkey.id().to_string(), wl_hotkey.description())
    }
}

impl From<WlNewHotKey> for NewShortcut {
    fn from(wl_hotkey: WlNewHotKey) -> Self {
        let mut ns = NewShortcut::new(wl_hotkey.id().to_string(), wl_hotkey.description());
        if let Some(pt) = wl_hotkey.preferred_trigger() {
            ns = ns.preferred_trigger(pt);
        }
        ns
    }
}
