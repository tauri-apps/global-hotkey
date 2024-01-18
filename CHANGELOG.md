# Changelog

## \[0.4.2]

- [`b538534`](https://www.github.com/tauri-apps/global-hotkey/commit/b538534f9181ccd38e76d93368378fc6ed3a3a08) Changed window class name used interally so it wouldn't conflict with `tray-icon` crate implementation.

## \[0.4.1]

- [`1f9be3e`](https://www.github.com/tauri-apps/global-hotkey/commit/1f9be3e0631817a9c96a4d98289158286cb689e8)([#47](https://www.github.com/tauri-apps/global-hotkey/pull/47)) Add support for `Code::Backquote` on Linux.
- [`1f9be3e`](https://www.github.com/tauri-apps/global-hotkey/commit/1f9be3e0631817a9c96a4d98289158286cb689e8)([#47](https://www.github.com/tauri-apps/global-hotkey/pull/47)) On Linux, fix hotkey `press/release` events order and sometimes missing `release` event when the modifiers have been already released before the key itself has been released.
- [`1f9be3e`](https://www.github.com/tauri-apps/global-hotkey/commit/1f9be3e0631817a9c96a4d98289158286cb689e8)([#47](https://www.github.com/tauri-apps/global-hotkey/pull/47)) On Linux, improve the performance of `GlobalHotKeyManager::register_all` and `GlobalHotKeyManager::unregister_all` to 2711x faster.

## \[0.4.0]

- [`53961a1`](https://www.github.com/tauri-apps/global-hotkey/commit/53961a1ade623bb97ce96db71fbe1193ffc9d6a7)([#35](https://www.github.com/tauri-apps/global-hotkey/pull/35)) Support Pressed and Released stats of the hotkey, you can check the newly added `state` field or using the `state()` method on the `GlobalHotKeyEvent`.

## \[0.3.0]

- [`fa47029`](https://www.github.com/tauri-apps/global-hotkey/commit/fa47029435ed953b07f5809d9e521bcd2c24bf54) Update `keyboard-types` to `0.7`

## \[0.2.4]

- [`b0975f9`](https://www.github.com/tauri-apps/global-hotkey/commit/b0975f9983aa023df3cd72bbd8d3158165e9f6eb) Export `CMD_OR_CTRL` const.
- [`dc9e619`](https://www.github.com/tauri-apps/global-hotkey/commit/dc9e6197362164ef6b8aae90df41a6a2b459f5fb) Add `GlobalHotKeyEvent::id` method.
- [`b960609`](https://www.github.com/tauri-apps/global-hotkey/commit/b96060952daf8959939f07c968b8bd58e33f4abd) Impl `TryFrom<&str>` and `TryFrom<String>` for `HotKey`.

## \[0.2.3]

- [`589ecd9`](https://www.github.com/tauri-apps/global-hotkey/commit/589ecd9afd79aab93b25b357b4c70afdf69f9f6d)([#25](https://www.github.com/tauri-apps/global-hotkey/pull/25)) Fix `GlobalHotKeyManager::unregister_all` actually registering the hotkeys instead of unregistering.

## \[0.2.2]

- [`bbd3ffb`](https://www.github.com/tauri-apps/global-hotkey/commit/bbd3ffbea2a76eaae7cd344a019a942456f94a26)([#23](https://www.github.com/tauri-apps/global-hotkey/pull/23)) Generate a hash-based id for hotkeys. Previously each hotkey had a unique id which is not necessary given that only one hotkey with the same combination can be used at a time.

## \[0.2.1]

- [`b503530`](https://www.github.com/tauri-apps/global-hotkey/commit/b503530eb49a7fe8da3e49080e3f72f82a70b7a2)([#20](https://www.github.com/tauri-apps/global-hotkey/pull/20)) Make `GlobalHotKeyManager` Send + Sync on macOS.

## \[0.2.0]

- Support more variants for `HotKey::from_str` and support case-insensitive htokey.
  - [25cbda5](https://www.github.com/tauri-apps/global-hotkey/commit/25cbda58c503b8230af00c6192e87d5ce1fc2742) feat: add more variants and case-insensitive hotkey parsing ([#19](https://www.github.com/tauri-apps/global-hotkey/pull/19)) on 2023-04-19

## \[0.1.2]

- On Windows, fix registering htokeys failing all the time.
  - [65d1f6d](https://www.github.com/tauri-apps/global-hotkey/commit/65d1f6dffd54bafe46d1ae776639b5dd10e78b96) fix(window): correctly check error result on 2023-02-13
- Fix crash on wayland, and emit a warning instead.
  - [4c08d82](https://www.github.com/tauri-apps/global-hotkey/commit/4c08d82fa4a20c82988b49f718688ec29de8a781) fix: emit error on non x11 window systems on 2023-02-13

## \[0.1.1]

- Update docs
  - [6409e5d](https://www.github.com/tauri-apps/global-hotkey/commit/6409e5dd351e1cae808c0042f4507e9afad70a05) docs: update docs on 2023-02-08

## \[0.1.0]

- Initial Release.
  - [72873f6](https://www.github.com/tauri-apps/global-hotkey/commit/72873f629b47565888d5f2a4264476c9974686b6) chore: add initial release change file on 2023-01-16
  - [d0f1d9c](https://www.github.com/tauri-apps/global-hotkey/commit/d0f1d9c58eba60015f658f7a742c200c2d1bd55e) chore: adjust change file on 2023-01-16
