// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

macro_rules! on_linux {
    () => {
        cfg!(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))
    };
}
macro_rules! on_linux_cfg {
    ($i:item) => {
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        $i
    };
}

macro_rules! not_on_linux_cfg {
    ($x:item) => {
        #[cfg(not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        $x
    };
}

pub(crate) use not_on_linux_cfg;
pub(crate) use on_linux;
pub(crate) use on_linux_cfg;
