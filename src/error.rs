// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use thiserror::Error;

use crate::hotkey::{HotKey, HotKeyParseError};

/// Errors returned by tray-icon.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    OsError(#[from] std::io::Error),
    #[error("{0}")]
    HotKeyParseError(HotKeyParseError),
    #[error("Unable to register hotkey: {0}")]
    FailedToRegister(String),
    #[error("Failed to unregister hotkey: {0:?}")]
    FailedToUnRegister(HotKey),
    #[error("HotKey already registered: {0:?}")]
    AlreadyRegistered(HotKey),
    #[error("Failed to watch media key event")]
    FailedToWatchMediaKeyEvent,
}

/// Convenient type alias of Result type for tray-icon.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::hotkey::HotKeyParseError;

    #[test]
    fn hotkey_parse_error_uses_typed_error() {
        let error = Error::HotKeyParseError(HotKeyParseError::EmptyToken("Shift++KeyA".into()));

        assert_eq!(
            error.to_string(),
            "Found empty token while parsing hotkey: Shift++KeyA"
        );
    }
}
