// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! This module is for Wayland-specific functions.
//!
//! # How Hotkeys on Wayland Work
//!
//! Wayland makes use of the XDG GlobalShortcuts portal ([see the official portal documentation for
//! more
//! details](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)).
//!
//! On Wayland, you define a set of actions, where each action has any number of associated
//! hotkeys to trigger it which are configured externally, not by the application.
//!
//! The first time some actions are registered, the user is shown a prompt with a list of actions,
//! a description for what each action does, and they are given an option to configure each action's
//! hotkeys (which may have a default setting). The user can configure these hotkeys later in their
//! system's settings.
//!
//! An application can request a list of each action along with the associated hotkeys, as well as
//! receive events whenever the user changes any of these hotkeys.
//!
//! # How to Use this Module
//!
//! You can verify if the user is using Wayland with the [`using_wayland`] function.
//!
//! Register all your actions using
//! [`GlobalHotKeyManager::wl_register_all`](crate::GlobalHotKeyManager::wl_register_all). This
//! should be used to register all your application's hotkey actions at once, since each call to
//! this function could create a popup.
//!
//! Use [`GlobalHotKeyManager::wl_get_hotkeys`](crate::GlobalHotKeyManager::wl_get_hotkeys) to get
//! the current list of registered hotkeys.
//!
//! The [`wl_have_hotkeys_changed`] function, or its blocking variant
//! [`wl_wait_for_hotkey_change`], will return true if the user has changed a hotkey since the last
//! call to either of these functions.
//!
//! Unregister hotkey actions using
//! [`GlobalHotKeyManager::wl_unregister_all`](crate::GlobalHotKeyManager::wl_unregister_all).
//!
//! # Notes
//!
//! - If you can't register any shortcuts, make sure:
//!     - You are running an xdg-desktop-portal backend that supports global shortcuts.
//!     - The app id is set correctly. See the documentation for [`GlobalHotKeyManager::wl_register_all`](crate::GlobalHotKeyManager::wl_register_all) for more information.
//!
//! # Example
//!
//! ```no_run
//! use global_hotkey::{
//!     hotkey::{Code, HotKey, Modifiers},
//!     wayland::{WlNewHotKeyAction, WlHotKeysChangedEvent},
//!     GlobalHotKeyEvent, GlobalHotKeyManager,
//! };
//!
//! const MY_ACTION_ID: u32 = 1;
//!
//! fn main() {
//!     // initialize hotkey manager
//!     let hotkey_manager = GlobalHotKeyManager::new().unwrap();
//!
//!     // registering an action with CTRL+META+O as the preferred hotkey
//!     let my_action = WlNewHotKeyAction::new(
//!         MY_ACTION_ID,
//!         "Do cool stuff.",
//!         Some(HotKey::new(
//!             Some(Modifiers::CONTROL | Modifiers::META),
//!             Code::KeyO,
//!         )),
//!     );
//!
//!     // register all your application's hotkey actions
//!     hotkey_manager.wl_register_all("com.github.example.ExampleAppID", &[my_action]).unwrap();
//!
//!     // listening to change hotkey events on another thread like how you would listen to hotkey
//!     // events.
//!     std::thread::spawn(move || {
//!         let Some(receiver) = WlHotKeysChangedEvent::receiver() else {
//!             return;
//!         };
//!         while let Ok(new_hotkeys) = receiver.recv() {
//!             println!(
//!                 "Some hotkeys were changed, here is what changed: {:?}",
//!                 hotkey_manager.wl_get_hotkeys()
//!             );
//!         }
//!     });
//!
//!     // receiving global hotkey events (i.e. hotkey presses/releases) on main thread
//!     let event_receiver = GlobalHotKeyEvent::receiver();
//!     while let Ok(event) = event_receiver.recv() {
//!         println!("{event:?}");
//!     }
//! }
//! ```

use crossbeam_channel::Receiver;
use std::{env, num::ParseIntError};

use ashpd::desktop::global_shortcuts::Shortcut;

use crate::{
    hotkey::HotKey,
    macros::{not_on_linux_cfg, on_linux_cfg},
    on_linux,
};

on_linux_cfg! {
    use crate::platform_impl::wl_hotkeys_changed_receiver;
}

/// Returns `true` if `WAYLAND_DISPLAY` is set and running on Linux/BSD.
pub fn using_wayland() -> bool {
    on_linux!() && env::var("WAYLAND_DISPLAY").is_ok()
}

pub struct WlHotKeysChangedEvent {
    pub changed_hotkeys: Vec<WlChangedHotKey>,
}

impl WlHotKeysChangedEvent {
    /// Gets receiver for WlHotKeysChangedEvent, which will allow you to listen to any changes the
    /// user makes to the registered hotkeys.
    ///
    /// Will return `None` if not using Linux and Wayland.
    pub fn receiver() -> Option<Receiver<WlHotKeysChangedEvent>> {
        Self::receiver_impl()
    }

    on_linux_cfg! {
        fn receiver_impl() -> Option<Receiver<WlHotKeysChangedEvent>> {
            Some(wl_hotkeys_changed_receiver()).filter(|_| using_wayland())
        }
    }

    not_on_linux_cfg! {
        fn receiver_impl() -> Receiver<WlHotKeysChangedEvent> {
            None
        }
    }
}

pub struct WlChangedHotKey {
    pub id: u32,
    pub hotkey_description: String,
}

impl TryFrom<Shortcut> for WlChangedHotKey {
    type Error = ParseIntError;

    fn try_from(value: Shortcut) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id().parse::<u32>()?,
            hotkey_description: value.trigger_description().into(),
        })
    }
}

/// Used to register a new action under Wayland which can have associated hotkeys.
#[derive(Debug, Clone)]
pub struct WlNewHotKeyAction {
    id: u32,
    description: String,
    preferred_hotkey: Option<HotKey>,
}

impl WlNewHotKeyAction {
    /// Creates a new [`WlNewHotKeyAction`].
    ///
    /// # Arguments
    ///
    /// * `id` - a unique [`u32`] to identify this action and all its associated hotkeys.
    /// * `description` - a short, human-readable description detailing what triggering this action does.
    /// * `preferred_hotkey` - an optional recommended hotkey that the user will be presented with
    ///   when registering this action for the first time. If the hotkey cannot be parsed, it will be
    ///   ignored.
    pub fn new<S>(id: u32, description: S, preferred_hotkey: Option<HotKey>) -> Self
    where
        S: Into<String>,
    {
        Self {
            id,
            description: description.into(),
            preferred_hotkey,
        }
    }

    /// A unique numerical id to identify the hotkeys associated with this action.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// A human-readable description detailing what triggering this action does.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The optional recommended key-combination that the user will be presented with when
    /// registering this hotkey for the first time.
    pub fn preferred_hotkey(&self) -> Option<HotKey> {
        self.preferred_hotkey
    }
}

/// A registered hotkey action that can have any number of associated hotkeys.
#[derive(Debug, Clone)]
pub struct WlHotKeyAction {
    pub(crate) id: u32,
    pub(crate) action_description: String,
    pub(crate) hotkey_description: String,
}

impl WlHotKeyAction {
    /// A unique numerical id to identify this action and its associated hotkeys.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// A human-readable description detailing what the action does.
    pub fn action_description(&self) -> &str {
        &self.action_description
    }

    /// Description of the hotkeys to trigger this action (e.g. `CTRL+ALT+U`).
    ///
    /// ## Note
    ///
    /// It can contain any number of hotkeys, including none at all. See the [shortcuts XDG
    /// specification](https://specifications.freedesktop.org/shortcuts-spec/latest/) for more
    /// information about how each hotkey is formatted.
    pub fn hotkey_description(&self) -> &str {
        &self.hotkey_description
    }
}
