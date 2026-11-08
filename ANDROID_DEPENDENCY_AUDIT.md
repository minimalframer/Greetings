# Android dependency audit

Audited against the current `Cargo.toml`, `src/main.rs`, and `src/lib.rs` on the `android/port-foundation` branch.

## Findings

### UI / application entry

- The package currently builds a desktop executable: `[[bin]] name = "zapfast"`, `path = "src/main.rs"`.
- The library is named `zapfast`; it is not configured as an Android `cdylib`.
- `src/main.rs` is explicitly a desktop entry point. It uses Clap command-line arguments, tray/single-instance control, desktop window recovery, desktop titlebar integration, and the `eframe::App` desktop shell.
- `eframe` is pinned to a patched egui/winit stack. Its currently enabled features are `accesskit`, `glow`, `default_fonts`, `links`, `wayland`, `x11`, and `persistence`. Neither Android activity backend is enabled.
- The patched winit revision must be checked for compatibility with the Android feature path; do not assume the upstream docs automatically describe this fork.

### Platform integrations that cannot be enabled unchanged on Android

The manifest currently declares desktop integrations unconditionally, including:
- `rfd` with `xdg-portal` and `wayland`
- `arboard` with `wayland-data-control`
- `open`
- many `fastframe-*` crates for tray, desktop shell, titlebar, autostart, dock, desktop notifications, power and desktop text/font integration
- Linux-only, macOS-only, and Windows-only dependencies in target-specific sections

These need to be feature-gated or moved into target-specific dependency sections, after checking which shared modules import them.

### High-risk native / system dependencies

- `rusqlite` enables `bundled-sqlcipher-vendored-openssl`. Android cross-compilation and SQLCipher/OpenSSL toolchain support must be validated; encrypted storage must remain encrypted.
- `whatsapp-rust` is built from a pinned Git revision with SQLite storage, Tokio transport/runtime, ureq client, native Tokio and VoIP features.
- Media/call functionality adds OpenH264, a pinned MP4 Git dependency, a pinned Rodio Git dependency, Opus, Ogg, Symphonia, WebP and audio-processing crates. These need an Android ABI build audit and Android-native audio/camera backends before call features can be enabled.
- `gethostname`, `directories`, `keyring-core`, `reqwest`, and `ureq` also need Android-specific runtime/storage/security review.

## Recommended implementation order

1. Preserve the desktop build as the default feature set.
2. Add a clearly separated `android` feature and a dedicated Android library entry point; do not compile the desktop `main.rs` for Android.
3. Configure the exact Winit Android activity backend through the existing pinned eframe/winit dependency graph. Winit 0.30 uses `android-activity 0.6`; the eframe feature surface exposes `android-native-activity` and `android-game-activity`. Verify the actual pinned fork and version before selecting one.
4. Split desktop-only dependencies and module imports behind `cfg(not(target_os = "android"))` or Cargo features.
5. Build the smallest Android target/ABI matrix first, then progressively re-enable storage, protocol, media and calling functionality.
6. Add APK packaging and CI only after a minimal Android activity builds and launches.

## Important limitation

This audit is a manifest/source inspection, not a successful Android compile. No Android SDK/NDK build has been run yet, and no APK should be advertised as available.
