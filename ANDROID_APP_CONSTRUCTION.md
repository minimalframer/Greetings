# Android app construction boundary

This note records the constructor and frame-loop boundary verified in `src/app.rs` on this branch. It is intentionally a design checkpoint, not an assertion that Android can launch the app yet.

## What the current constructor actually does

`App::new(waker, dirs, settings, options)` currently combines two different jobs:

1. Shared application startup:
   - configures the WhatsApp proxy;
   - loads the account roster and starts each account/backend;
   - creates the app state and file writer;
   - loads custom themes and applies settings;
   - sends initial download-folder and archive preferences to each backend.
2. Desktop process services:
   - creates a desktop notification badge through `fastframe_desktop::desktop_entry`;
   - enables desktop themes;
   - follows desktop text-rendering preferences;
   - spawns desktop power monitoring;
   - optionally spawns Linux system appearance;
   - creates the tray and macOS Dock integration;
   - reads the desktop login item.

The `AppOptions { tray: bool }` flag only controls tray-related behavior; it does not make `App::new` platform-neutral.

## Existing reusable seam

`App::headless(dirs, settings)` creates a disconnected app for tests and demos. It uses `Account::detached` and does not start a WhatsApp connection, so it is **not** a production Android constructor.

`App::background_frame(ctx)` is already public and documents itself as processing app state shared by windowed and headless modes. However, it still calls desktop-oriented helpers and fields (power, login item, tray, badge, control commands, and desktop theme handling). It is not yet a clean Android lifecycle frame.

`App` implements `fastframe_shell::Resident` in `app.rs`, which couples the app type itself to the desktop resident-shell abstraction. This is a deeper boundary than the private adapter in `main.rs`.

## Safe extraction plan

Do not simply move the body of `App::new` into a new file: it initializes fields and services that are consumed throughout the 1 MB app implementation.

The first safe code refactor should be split into explicit stages while preserving the existing desktop path:

1. Introduce a private/shared constructor that loads settings, roster, accounts and creates the common app state.
2. Keep desktop service startup in a desktop-only initializer called by the existing `App::new`, preserving badge, power, appearance, tray, Dock, login item and desktop theme behavior.
3. Define an Android service initializer separately. It must supply Android implementations for notifications, power/lifecycle, storage, file selection, clipboard, audio focus, camera and external URI handling before production use.
4. Move `fastframe_shell::Resident` behavior behind a desktop adapter/feature; do not let Android depend on desktop shell crates.
5. Keep `background_frame` shared only after each desktop service call has been routed through a platform service interface or a safe no-op with documented semantics.

## Current blockers

- `App` stores desktop service types directly (including fastframe file writer, clipboard reader, power, tray, login item and badge).
- `App::new` starts real account backends; `App::headless` deliberately does not.
- The frame loop handles window focus, tray events, desktop power events and remote-control commands.
- No Android storage/key provider, picker, notification channel, audio-focus or camera implementation has been added.
- No Android launcher or Android-target build has been verified.

## Acceptance checks for the refactor

- Existing desktop startup, tray, login-item, notification badge, power-resume and shutdown behavior remains unchanged.
- Unit tests using `App::headless` continue to pass.
- A desktop build succeeds with the default feature set.
- Android code does not link `fastframe_shell`, `fastframe_tray`, `fastframe_desktop`, or desktop login-item services.
- SQLCipher configuration and key handling remain intact.
- Only after the above: build the Android target and test activity recreation, pause/resume, back navigation, keyboard, picker, clipboard and notification flows.
