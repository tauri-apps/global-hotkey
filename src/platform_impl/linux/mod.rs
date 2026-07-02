// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use crossbeam_channel::{unbounded, Sender};

use crate::hotkey::HotKey;

mod wayland;
mod x11;

pub(crate) enum ThreadMessage {
    RegisterHotKey(HotKey, Sender<crate::Result<()>>),
    RegisterHotKeys(Vec<HotKey>, Sender<crate::Result<()>>),
    UnRegisterHotKey(HotKey, Sender<crate::Result<()>>),
    UnRegisterHotKeys(Vec<HotKey>, Sender<crate::Result<()>>),
    DropThread,
}

/// The event loop thread exited (e.g. neither the Wayland portal nor X11 was
/// reachable), so requests can no longer be processed.
fn backend_thread_dead() -> crate::Error {
    crate::Error::OsError(std::io::Error::other(
        "global hotkey backend thread is not running",
    ))
}

fn is_wayland() -> bool {
    static IS_WAYLAND: OnceLock<bool> = OnceLock::new();
    *IS_WAYLAND.get_or_init(|| {
        std::env::var("WAYLAND_DISPLAY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    })
}

pub struct GlobalHotKeyManager {
    thread_tx: Sender<ThreadMessage>,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        let (thread_tx, thread_rx) = unbounded();
        std::thread::spawn(move || {
            let use_wayland = is_wayland()
                && std::env::var("GDK_BACKEND")
                    .map(|v| v != "x11")
                    .unwrap_or(true);

            let result = if use_wayland {
                match wayland::events_processor(thread_rx) {
                    // Portal missing at startup: fall back to X11 with the same receiver
                    // so no pending register messages are lost.
                    Ok(wayland::Outcome::Unavailable(thread_rx)) => {
                        x11::events_processor(thread_rx)
                    }
                    Ok(wayland::Outcome::Handled) => Ok(()),
                    Err(err) => Err(err),
                }
            } else {
                x11::events_processor(thread_rx)
            };
            if let Err(_err) = result {
                #[cfg(feature = "tracing")]
                tracing::error!("{}", _err);
            }
        });
        Ok(Self { thread_tx })
    }

    pub fn register(&self, hotkey: HotKey) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::RegisterHotKey(hotkey, tx));

        rx.recv().map_err(|_| backend_thread_dead())?
    }

    pub fn unregister(&self, hotkey: HotKey) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::UnRegisterHotKey(hotkey, tx));

        rx.recv().map_err(|_| backend_thread_dead())?
    }

    pub fn register_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::RegisterHotKeys(hotkeys.to_vec(), tx));

        rx.recv().map_err(|_| backend_thread_dead())?
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::UnRegisterHotKeys(hotkeys.to_vec(), tx));

        rx.recv().map_err(|_| backend_thread_dead())?
    }
}

impl Drop for GlobalHotKeyManager {
    fn drop(&mut self) {
        let _ = self.thread_tx.send(ThreadMessage::DropThread);
    }
}
