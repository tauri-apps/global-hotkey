use ashpd::desktop::{
    global_shortcuts::{Activated, Deactivated, GlobalShortcuts, NewShortcut, ShortcutsChanged},
    Session,
};
use crossbeam_channel::{Receiver, Select, Sender};
use futures::{stream::select_all, Stream, StreamExt};

use crate::{
    error::Error, platform_impl::platform::ThreadMessage, wayland::WlHotKey, GlobalHotKeyEvent,
    HotKeyState,
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

    let gs_event_stream: Box<dyn Stream<Item = GSEvent> + Unpin + Send> = {
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
    tokio::spawn(recv_gs_events(gs_event_stream, tx));

    let mut select = Select::new();
    let thread_rx_idx = select.recv(&thread_rx);
    let gs_rx_idx = select.recv(&gs_rx);
    loop {
        println!("Here");
        let selected_oper = select.select();
        match selected_oper.index() {
            i if i == thread_rx_idx => {
                if let Ok(msg) = selected_oper.recv(&thread_rx) {
                    match msg {
                        ThreadMessage::WlRegisterHotKeys(hotkeys, tx) => {
                            let new_hotkeys = hotkeys
                                .into_iter()
                                .filter(|hk| {
                                    !registered_hotkeys.iter().any(|rhk| rhk.id() == hk.id())
                                })
                                .collect::<Vec<WlHotKey>>();
                            registered_hotkeys.extend_from_slice(&new_hotkeys);

                            let _ = tx.send(
                                reregister_hotkeys(&proxy, &mut session, &registered_hotkeys).await,
                            );
                        }
                        ThreadMessage::DropThread => return Ok(()),
                        _ => {}
                    }
                }
            }
            i if i == gs_rx_idx => {
                if let Ok(msg) = selected_oper.recv(&gs_rx) {
                    match msg {
                        GSEvent::Activated(activated) => {
                            if let Ok(id) = activated.shortcut_id().parse::<u32>() {
                                GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                                    id,
                                    state: HotKeyState::Pressed,
                                });
                            }
                        }
                        GSEvent::Deactivated(deactivated) => {
                            if let Ok(id) = deactivated.shortcut_id().parse::<u32>() {
                                GlobalHotKeyEvent::send(GlobalHotKeyEvent {
                                    id,
                                    state: HotKeyState::Released,
                                });
                            }
                        }
                        // TODO: handle changed events and pass it along to user
                        GSEvent::Changed(_) => {}
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

async fn recv_gs_events(
    mut stream: Box<dyn Stream<Item = GSEvent> + Unpin + Send>,
    tx: Sender<GSEvent>,
) {
    while let Some(ev) = stream.next().await {
        let _ = tx.send(ev);
    }
}

async fn reregister_hotkeys<'a>(
    proxy: &GlobalShortcuts<'a>,
    session: &mut Session<'a, GlobalShortcuts<'a>>,
    hotkeys: &[WlHotKey],
) -> Result<(), Error> {
    session.close().await.map_err(|e| {
        Error::FailedToRegister(format!("Failed to close old global shortcuts session: {e}"))
    })?;

    *session = proxy.create_session().await.map_err(|e| {
        Error::FailedToRegister(format!(
            "Failed to start global shortcuts portal session: {e}"
        ))
    })?;

    let hotkeys_to_register = hotkeys
        .iter()
        .map(|hk| {
            if let Some(preferred_trigger) = hk.preferred_trigger() {
                NewShortcut::new(hk.id().to_string(), hk.description())
                    .preferred_trigger(preferred_trigger)
            } else {
                NewShortcut::new(hk.id().to_string(), hk.description())
            }
        })
        .collect::<Vec<NewShortcut>>();

    proxy
        .bind_shortcuts(session, &hotkeys_to_register, None)
        .await
        .map_err(|e| Error::FailedToRegister(format!("Call to BindShortcuts failed: {e}")))?;
    Ok(())
}
