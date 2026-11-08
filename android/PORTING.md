# Android port status

The Android port is being developed on the `android/greetings-apk` branch. The desktop application and the Android application must share the WhatsApp protocol, account, archive, and UI logic; desktop-only process services must not be started on Android.

## Verified source constraints

- `src/main.rs` is a desktop CLI and shell launcher. It uses Clap, single-instance control, desktop logging, tray integration, and `fastframe_shell::Shell`.
- `App::new` in `src/app.rs` starts account backends and then calls `initialize_desktop_services`.
- `App::headless` creates a detached account and is intended for tests/demos. It is not a connected WhatsApp application and must not be used as the production Android startup path.
- The shared app state still contains desktop service types, and the frame loop calls desktop lifecycle, notification, audio, and video handling.
- The current `android-game-activity` Cargo feature only selects eframe's Android backend. It does not provide an Android entry point, an Activity, or Android implementations of desktop services.

## Work remaining

1. Split the application host from shared app state. Android startup must construct a real account backend, not the detached backend used by `App::headless`.
2. Introduce platform-neutral service interfaces for notifications, clipboard, file picking/opening, audio, camera, app lifecycle, and secure key storage. Keep existing desktop implementations behind desktop target configuration.
3. Add the Android GameActivity host, manifest, Gradle project, and JNI/native entry point. Keep the user-visible application label as **Greetings**.
4. Configure Android app-private directories and secure key storage. Confirm SQLCipher/OpenSSL and WhatsApp transport dependencies build for arm64 and other intended ABIs.
5. Implement Android notification permission/channel handling, background reconnect policy, media picker, share/open intents, microphone/camera permissions, and lifecycle pause/resume.
6. Run device tests for QR pairing, reconnect, one-to-one and group messaging, media transfer, history, encrypted persistence, notifications, and calls before publishing an APK.

## Automated compile gate

The Android workflow checks the Rust library for `aarch64-linux-android` with the GameActivity feature. A successful `cargo check` is only a compile milestone; it does not prove that the Activity launches or that WhatsApp features work. The APK is not considered complete until device tests pass.

## Current status

No Android APK has yet been built or device-tested. Do not describe the current branch as a working Android client.
