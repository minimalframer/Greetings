//! Desktop resident-shell integration for the application.
//!
//! This adapter deliberately stays separate from the shared app state. It is
//! still compiled for the desktop build; a later platform split can gate this
//! module and replace it with an Android lifecycle host.

use std::time::{Duration, Instant};

use crate::backend::{Command, Waker};

use super::{
    hyprland, tray_colour, tray_config, wants_tray_item, App, AppOptions, TRAY_COLOUR_EVERY,
    TRAY_LOCK, TRAY_WAIT,
};


impl App {
    /// Starts process services owned by the desktop host.
    ///
    /// Kept separate from account/app-state construction so a mobile host can
    /// provide its own lifecycle and platform services without starting a
    /// desktop tray, Dock item, login item, or desktop appearance watchers.
    /// This is an extraction boundary only: App still stores desktop service
    /// types, so it is not yet an Android initializer.
    pub(super) fn initialize_desktop_services(&mut self, waker: &Waker, options: AppOptions) {
        self.pauses_media = true;
        self.call_tones = crate::call_tones::Tones::new(true);
        self.desktop_services.badge = Some(crate::notify::Badge::new(fastframe_desktop::desktop_entry(
            "zapfast",
        )));
        self.custom_themes
            .enable_desktop_themes(crate::theme::DESKTOP_THEMES);
        self.load_custom_themes();
        // Reading the desktop's font settings may wait on D-Bus; keep it off
        // the first frame.
        let text_waker = waker.clone();
        std::thread::Builder::new()
            .name("text-rendering".into())
            .spawn(move || crate::theme::follow_text_rendering(move || text_waker.wake()))
            .ok();
        let power_waker = waker.clone();
        self.desktop_services.power = fastframe_power::Power::spawn(move || power_waker.wake());
        #[cfg(target_os = "linux")]
        {
            let appearance_waker = waker.clone();
            self.desktop_services.system_appearance = Some(crate::appearance::SystemAppearance::spawn(move || {
                appearance_waker.wake()
            }));
        }
        self.desktop_services.tray_allowed = options.tray;
        self.spawn_tray_if_wanted();
        if cfg!(target_os = "macos") && options.tray {
            let dock_waker = waker.clone();
            self.desktop_services.dock = Some(fastframe_tray::Dock::spawn(move || dock_waker.wake()));
        }
        // The clock preference may wait on the desktop portal on Linux; keep
        // it off the first frame.
        std::thread::Builder::new()
            .name("clock-format".into())
            .spawn(|| {
                crate::util::twelve_hour_clock();
            })
            .ok();
        let login_waker = waker.clone();
        self.desktop_services.login_item.read(move || login_waker.wake());
    }


    /// Reports whether the desktop says the display is off.
    ///
    /// Shared notification and presence logic consumes this narrow signal
    /// rather than depending directly on the desktop power service.
    pub(super) fn desktop_display_off(&self) -> bool {
        self.desktop_services.power.conditions().display_off
    }

    /// Highlights the taskbar entry for a message that notified while the
    /// window is open behind others, as KDE Plasma's task manager shows for
    /// chat apps. The desktop clears it once the window is focused. Linux
    /// only: elsewhere it would bounce the Dock or flash the taskbar. Asks the
    /// viewport too, since `window_focused` stays false behind the app lock
    /// even while the lock screen has the focus.
    pub(super) fn request_attention(&mut self, ctx: &egui::Context) {
        let skip = fastframe_desktop::is_gnome()
            || hyprland(
                std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some(),
                std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
            );
        self.request_attention_on(ctx, skip);
    }


    fn handle_power(&mut self) {
        for event in self.desktop_services.power.events() {
            match event {
                fastframe_power::Event::Suspending(ack)
                | fastframe_power::Event::EndingSession(ack) => {
                    self.save_state();
                    // Within the time the system waits for the app.
                    self.files.flush(Duration::from_millis(1500));
                    ack.done();
                }
                fastframe_power::Event::Resumed => {
                    for account in &self.accounts {
                        account.backend.send(Command::Resumed);
                    }
                }
            }
        }
    }


    fn tick_login_item(&mut self) {
        let Some(outcome) = self.desktop_services.login_item.poll() else {
            return;
        };
        match outcome.state {
            Ok(state) => {
                self.start_with_system = crate::autostart::switch(state);
                if outcome.changed && matches!(state, fastframe_autostart::State::NeedsApproval(_))
                {
                    self.toast("Allow ZapFast in System Settings, under Login Items.");
                }
            }
            Err(error) => {
                if outcome.changed {
                    self.toast_error(format!("Could not change the login item: {error}"));
                }
            }
        }
    }

    /// Runs integrations owned by the desktop host, outside shared frame work.
    ///
    /// Keeping these calls together gives the Android host a clear exclusion
    /// boundary without changing the app's underlying service fields yet.
    pub(super) fn handle_desktop_frame(&mut self) {
        self.tick_login_item();
        self.handle_power();
        self.handle_tray();
        #[cfg(target_os = "macos")]
        self.actions.extend(crate::macos::drain(self.window_hidden));
        self.handle_control_commands();
        self.poll_custom_themes();
    }
}

/// The app outlives its window: closing it with "keep running" on hides
/// ZapFast, and the tray, a notification or another launch brings it back.
/// Starts or synchronizes the desktop tray and its menu-bar appearance.
    ///
    /// Kept in the desktop adapter so shared app logic does not need to
    /// implement tray creation, Dock attachment, or taskbar icon updates.
    /// Makes the tray item once "Keep running when the window closes" asks
    /// for it: only then does it have a window to bring back (#457).
    fn spawn_desktop_tray_if_wanted(&mut self) {
        if !self.settings.keep_running_in_background {
            self.desktop_services.tray_failed = false;
        }
        if self.desktop_services.tray_failed
            || !wants_tray_item(
                self.desktop_services.tray_allowed,
                self.settings.keep_running_in_background,
                self.desktop_services.tray.is_some(),
            )
        {
            return;
        }
        let waker = self.waker.clone();
        self.desktop_services.tray =
            fastframe_tray::Tray::spawn(tray_config(self.tray_lockable), move || waker.wake());
        // No session bus, say: asked again only if the setting is turned
        // back on, not on every frame.
        self.desktop_services.tray_failed = self.desktop_services.tray.is_none();
        self.desktop_services.tray_badge = 0;
        self.desktop_services.tray_colour_checked = None;
    }

    /// Offers "Lock ZapFast" in the tray exactly while an app lock password
    /// is set, so setting or removing one changes the menu at once, and
    /// makes the item when the setting that wants it is turned on.
    fn sync_desktop_tray(&mut self) {
        if self.desktop_services.tray.is_none() {
            self.spawn_desktop_tray_if_wanted();
            if !self.window_hidden
                && let Some(tray) = &mut self.desktop_services.tray
            {
                // Makes the macOS item, which needs a running window.
                tray.attach();
            }
        }
        let lockable = self.settings.app_lock_hash.is_some();
        if lockable != self.tray_lockable {
            self.tray_lockable = lockable;
            if let Some(tray) = &mut self.desktop_services.tray {
                tray.set_visible(TRAY_LOCK, lockable);
            }
        }
        self.sync_desktop_tray_colour(Instant::now());
        // The same count as the taskbar's; a new icon only when it shows.
        let count = self.unread_chat_count_everywhere();
        let level = crate::util::tray_badge_level(count);
        if level != self.desktop_services.tray_badge
            && let Some(tray) = &mut self.desktop_services.tray
        {
            let (icon, template) = crate::util::tray_icons(count);
            tray.set_icon(icon, Some(template));
            self.desktop_services.tray_badge = level;
        }
    }

    /// Keeps the tray icon monochrome against the panel, as the macOS menu
    /// bar does by itself (#518). Windows has no signal without a window, so
    /// this asks every few seconds.
    fn sync_desktop_tray_colour(&mut self, now: Instant) {
        if self
            .desktop_services.tray_colour_checked
            .is_some_and(|checked| now.duration_since(checked) < TRAY_COLOUR_EVERY)
        {
            return;
        }
        self.desktop_services.tray_colour_checked = Some(now);
        let colour = tray_colour(self.panel_is_dark());
        if let Some(tray) = &mut self.desktop_services.tray {
            tray.set_icon_colour(colour);
        }
    }

    /// Whether the desktop's panel is dark, where that can be known. GNOME's
    /// top bar is dark in light mode too; other Linux panels follow the
    /// desktop's colour scheme.
    #[cfg(target_os = "linux")]
    fn panel_is_dark(&self) -> Option<bool> {
        if fastframe_desktop::is_gnome() {
            return Some(true);
        }
        self.desktop_services.system_appearance.as_ref()?.dark()
    }

    /// Whether the taskbar is dark, which Windows sets apart from the apps'
    /// light or dark mode.
    #[cfg(windows)]
    fn panel_is_dark(&self) -> Option<bool> {
        crate::util::windows_taskbar_dark()
    }

    /// The macOS menu bar recolours its template icon by itself.
    #[cfg(not(any(target_os = "linux", windows)))]
    fn panel_is_dark(&self) -> Option<bool> {
        None
    }

impl fastframe_shell::Resident for App {
    fn closed(&self) -> fastframe_shell::Closed {
        if self.quit_requested {
            fastframe_shell::Closed::Quit
        } else if self.reopen {
            // The window was closed only to make a fresh one, which is how a
            // Wayland session brings a minimized or covered window forward.
            fastframe_shell::Closed::Reopen
        } else if self.hide_intent {
            fastframe_shell::Closed::Hide
        } else {
            fastframe_shell::Closed::Quit
        }
    }

    fn window_gone(&mut self) {
        App::window_gone(self);
    }

    fn headless_frame(&mut self, ctx: &egui::Context) -> fastframe_shell::Headless {
        self.background_frame(ctx);
        self.check_tray_wait(Instant::now());
        if self.quit_requested {
            fastframe_shell::Headless::Quit
        } else if self.wants_show {
            fastframe_shell::Headless::Show
        } else {
            fastframe_shell::Headless::Wait
        }
    }

    /// Without a tray there is no way back to a hidden window, so show it.
    /// A tray item no panel shows yet may only be early: a start at login
    /// can beat the desktop's tray watcher (GNOME's AppIndicator extension
    /// loads after the session's autostart apps), so the window waits
    /// `TRAY_WAIT` for a panel before it shows.
    fn start_hidden(&mut self) -> bool {
        if !self.has_tray() || !self.settings.keep_running_in_background {
            return false;
        }
        if !self.tray_shown() {
            self.tray_wait = Some(Instant::now() + TRAY_WAIT);
        }
        App::start_hidden(self);
        true
    }

    fn shutdown(&mut self) {
        App::shutdown(self);
    }
}
