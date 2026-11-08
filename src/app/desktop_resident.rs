//! Desktop resident-shell integration for the application.
//!
//! This adapter deliberately stays separate from the shared app state. It is
//! still compiled for the desktop build; a later platform split can gate this
//! module and replace it with an Android lifecycle host.

use std::time::{Duration, Instant};

use crate::backend::Command;

use super::{hyprland, App, TRAY_WAIT};


impl App {
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
        for event in self.power.events() {
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

