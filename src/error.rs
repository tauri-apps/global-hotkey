// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::hotkey::HotKey;

/// Errors returned by tray-icon.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    OsError(#[from] std::io::Error),
    #[error("{0}")]
    HotKeyParseError(String),
    #[error("{0}")]
    FailedToRegister(String),
    #[error("Failed to unregister this hotkey")]
    FailedToUnRegister,
    #[error("HotKey already registerd: {0:?}")]
    AlreadyRegistered(HotKey),
}

/// Convenient type alias of Result type for tray-icon.
pub type Result<T> = std::result::Result<T, Error>;
