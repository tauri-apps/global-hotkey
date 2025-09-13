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
    ($i:block) => {
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

macro_rules! on_linux_cfg_item {
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
    ($i:block) => {
        #[cfg(not(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        $i
    };
}

pub(crate) use not_on_linux_cfg;
pub(crate) use on_linux;
pub(crate) use on_linux_cfg;
pub(crate) use on_linux_cfg_item;
