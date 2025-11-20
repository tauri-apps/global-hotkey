// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crossbeam_channel::{unbounded, Sender};

use crate::{
    hotkey::HotKey,
    wayland::{using_wayland, WlHotKeyAction, WlNewHotKeyAction},
};

mod wayland;
mod x11;

pub(crate) use wayland::wl_hotkeys_changed_receiver;

enum ThreadMessage {
    WlRegisterHotKeys(Vec<WlNewHotKeyAction>, String, Sender<crate::Result<()>>),
    WlUnRegisterHotKeys(Vec<u32>),
    WlGetHotKeys(Sender<Box<[WlHotKeyAction]>>),

    RegisterHotKey(HotKey, Sender<crate::Result<()>>),
    RegisterHotKeys(Vec<HotKey>, Sender<crate::Result<()>>),
    UnRegisterHotKey(HotKey, Sender<crate::Result<()>>),
    UnRegisterHotKeys(Vec<HotKey>, Sender<crate::Result<()>>),
    DropThread,
}

pub struct GlobalHotKeyManager {
    thread_tx: Sender<ThreadMessage>,
}

impl GlobalHotKeyManager {
    pub fn new() -> crate::Result<Self> {
        let (thread_tx, thread_rx) = unbounded();
        std::thread::spawn(|| {
            if let Err(_err) = if using_wayland() {
                wayland::events_processor(thread_rx)
            } else {
                x11::events_processor(thread_rx)
            } {
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

    pub fn register_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::RegisterHotKeys(hotkeys.to_vec(), tx));

        if let Ok(result) = rx.recv() {
            result?;
        }

        Ok(())
    }

    pub fn unregister_all(&self, hotkeys: &[HotKey]) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self
            .thread_tx
            .send(ThreadMessage::UnRegisterHotKeys(hotkeys.to_vec(), tx));

        if let Ok(result) = rx.recv() {
            result?;
        }

        Ok(())
    }

    pub fn wl_register_all(
        &self,
        app_id: impl Into<String>,
        hotkeys: &[WlNewHotKeyAction],
    ) -> crate::Result<()> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self.thread_tx.send(ThreadMessage::WlRegisterHotKeys(
            hotkeys.to_vec(),
            app_id.into(),
            tx,
        ));

        if let Ok(result) = rx.recv() {
            result?;
        }

        Ok(())
    }

    pub fn wl_unregister_all(&self, ids: &[u32]) {
        let _ = self
            .thread_tx
            .send(ThreadMessage::WlUnRegisterHotKeys(ids.to_vec()));
    }

    pub fn wl_get_hotkeys(&self) -> Box<[WlHotKeyAction]> {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let _ = self.thread_tx.send(ThreadMessage::WlGetHotKeys(tx));

        if let Ok(result) = rx.recv() {
            result
        } else {
            Box::new([])
        }
    }
}

impl Drop for GlobalHotKeyManager {
    fn drop(&mut self) {
        let _ = self.thread_tx.send(ThreadMessage::DropThread);
    }
}
