# Android module boundary audit

This audit is based on the current `src/lib.rs`, `src/main.rs`, and the module implementations inspected on the `android/port-foundation` branch.

## Safe to keep in the shared core initially

These modules appear to contain protocol, data, rendering, or application logic rather than an inherently desktop-only entry point. They still need an Android-target compile check and transitive dependency review:

- `account`, `archive`, `backend`, `bidi`, `blocking`, `call_diag`
- `emoji`, `geo`, `group_admin`, `i18n`, `image_cache`, `image_preview`
- `lottie`, `markup`, `mentions`, `model`, `motion`, `nav`, `new_group`
- `privacy`, `proxy`, `qr`, `safety`, `settings`, `sticker_meta`, `sticker_search`
- `target`, `theme`, `transcript`, `transport`, `util`, `video_rate`, `voice`, `wallpaper`

“Shared” does not mean already Android-compatible: storage, codecs, keyring, and file APIs used inside these modules must still be audited.

## Desktop shell / platform integration candidates

These are the first modules to isolate behind a desktop feature or a platform interface. Do not simply remove them: UI and application code currently refer to several of them.

| Module | Current desktop coupling / evidence | Android direction |
|---|---|---|
| `autostart` | `fastframe_autostart`, login-item and tray semantics | Android app lifecycle; no desktop autostart module |
| `backdrop` | desktop window transparency/material APIs | no-op or Android window/system-bar implementation |
| `single_instance` | desktop lock, socket and TCP control channel | Android activity/process lifecycle; deep links via Intent |
| `opener` | launches desktop applications and file managers; Linux D-Bus | Android Intent / content URI handler |
| `appearance` | currently exported only on Linux | separate Android system appearance implementation if required |
| `macos` | macOS-specific | keep target-gated |
| `notify` | fastframe desktop notification and dock badge APIs | Android notification channels and notification actions |
| `updates` | desktop updater/install helper | Android package updates belong to the distribution channel |
| `paths` | `directories::ProjectDirs`, desktop runtime directory and legacy migrations | Android app-private files/cache plus Android-backed key storage |
| `camera` | platform camera backend currently documents V4L2/AVFoundation/Media Foundation | Android Camera2/CameraX bridge or a verified Android camera backend |
| `audio`, `call_audio`, `media_pause` | host audio device enumeration and desktop device lifecycle | Android audio focus, audio devices, microphone permission and lifecycle |

## Entry point findings

- `src/main.rs` is explicitly a desktop CLI. It parses desktop arguments, uses the single-instance guard, builds `fastframe_shell::Shell`, manages tray/window recovery, and constructs desktop `NativeOptions`.
- It must remain the desktop binary entry point. An Android entry point should be a separate target-specific module/binary path and must not call the desktop CLI.
- `app::App::new` currently takes a desktop `Waker` and `AppOptions { tray: true }`; this constructor contract needs to be split before an Android launcher can create the app.
- The UI is currently adapted through the private `Shell` eframe implementation in `main.rs`. Reusing the UI therefore requires extracting the eframe app adapter and separating window-shell services from the core app.

## First implementation sequence

1. Extract platform-neutral construction of `app::App` from the desktop-only `main.rs` path, preserving the current desktop constructor and behavior.
2. Define a small platform-services interface for wake, storage/key access, notifications, file/media picking, and external links.
3. Move desktop-only integrations behind a `desktop` feature, but only after all call sites have an interface or a platform-specific implementation.
4. Add Android app entry and lifecycle handling using the pinned eframe GameActivity backend.
5. Compile desktop and Android targets after each boundary change; do not weaken SQLCipher or keyring behavior to make a target compile.

## Explicitly not done

- No modules have been cfg-disabled or removed.
- No Android camera/audio/keyring implementation is claimed.
- No Android entry point, manifest, Gradle project, APK, or successful Android build exists yet.
