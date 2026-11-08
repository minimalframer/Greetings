//! Service handles owned by the desktop host rather than shared app state.
//!
//! The headless constructor supplies inert/fixed implementations for tests;
//! the desktop initializer replaces them with live process services.

pub(super) struct DesktopServices {
    /// The read or change of the login item running on its thread.
    pub(super) login_item: crate::autostart::LoginItem,
    /// Sleep and wake as the system reports them. Tests and demo runs get a
    /// fixed one that reads nothing from the machine.
    pub(super) power: fastframe_power::Power,
    /// Desktop taskbar badge; absent in headless tests and demos.
    pub(super) badge: Option<crate::notify::Badge>,
    /// System tray and macOS Dock integration owned by the desktop host.
    pub(super) tray: Option<fastframe_tray::Tray>,
    pub(super) tray_allowed: bool,
    pub(super) tray_failed: bool,
    pub(super) dock: Option<fastframe_tray::Dock>,
    pub(super) tray_badge: u32,
    pub(super) tray_colour_checked: Option<std::time::Instant>,
    #[cfg(target_os = "linux")]
    pub(super) system_appearance: Option<crate::appearance::SystemAppearance>,
}

impl Default for DesktopServices {
    fn default() -> Self {
        Self {
            login_item: Default::default(),
            power: fastframe_power::Power::fixed(Default::default()),
            badge: None,
            tray: None,
            tray_allowed: false,
            tray_failed: false,
            dock: None,
            tray_badge: 0,
            tray_colour_checked: None,
            #[cfg(target_os = "linux")]
            system_appearance: None,
        }
    }
}
