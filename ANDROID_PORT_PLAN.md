# Android port plan

## Current state

Greetings is currently a desktop fork of ZapFast, not an Android application yet.

Evidence in the current source:
- The binary entry point is `src/main.rs` and starts the desktop shell through `eframe`.
- The UI and application logic are exported from `src/lib.rs`.
- The manifest enables desktop window-system features (`wayland`, `x11`) and includes desktop integrations such as `rfd`, `arboard`, `open`, tray, desktop autostart, and desktop notifications.
- The WhatsApp protocol layer is provided by `whatsapp-rust`; it should be kept separate from platform-specific UI and operating-system integrations wherever possible.

## Porting strategy

Do not try to make the existing desktop binary run on Android by only adding an Android target. First establish a platform boundary, then add an Android application entry point.

### Phase 1 — Build and dependency audit
- Keep the existing desktop application working.
- Audit every direct dependency and every `#[cfg(target_os = ...)]` use for Android support.
- Check the selected `eframe/winit` versions and renderer for Android lifecycle and input support before choosing the Android entry point.
- Identify native dependencies that need Android NDK/SDK toolchains or Android-specific features (notably media codecs, SQLite/SQLCipher, OpenSSL, audio, camera, and image/video handling).

### Phase 2 — Separate shared core from desktop shell
- Keep WhatsApp session, protocol, archive, message models, and business logic in shared Rust modules.
- Move desktop-only startup, tray, autostart, window management, desktop file picker, clipboard, and external-app opening behind platform modules or feature flags.
- Add Android implementations for storage paths, secure key storage, notifications, media picking, camera/microphone permissions, and lifecycle handling.
- Do not silently replace encrypted storage or credential handling with weaker alternatives.

### Phase 3 — Android application shell
- Add a dedicated Android entry point and Android manifest/package metadata.
- Choose and pin a supported Android UI integration after the renderer audit (the existing egui UI may be reusable, but desktop shell assumptions must not leak into it).
- Implement foreground/background lifecycle, network reconnect, permission requests, back navigation, and secure local storage.
- Preserve the WhatsApp companion-device pairing flow and clearly communicate unofficial-client/account risks.

### Phase 4 — CI and validation
- Add an Android SDK/NDK-enabled GitHub Actions job only after the Android entry point and target-specific dependency graph exist.
- Build at least one debug APK first; add release signing only through repository secrets and never commit signing keys.
- Run Rust formatting/tests and retain the existing desktop build as a regression check.
- Install and smoke-test the APK on an emulator and a physical device before calling the port usable.

## Immediate blockers / next engineering tasks

1. Inventory all platform-specific modules and dependencies.
2. Confirm the exact Android support matrix for the pinned egui/winit and native crates.
3. Define the Android package ID, minimum Android API level, and UI approach.
4. Implement the first Android shell without changing the existing desktop entry point.

## Scope note

An APK workflow added before these prerequisites would likely fail at compile/link time or produce an app without a functioning Android lifecycle. This plan deliberately avoids presenting a speculative workflow as a working Android build.
