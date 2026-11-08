//! Service handles owned by the desktop host rather than shared app state.
//!
//! The headless constructor supplies inert/fixed implementations for tests;
//! the desktop initializer replaces them with live process services.

pub(super) struct DesktopServices {
    pub(super) login_item: crate::autostart::LoginItem,
    pub(super) power: fastframe_power::Power,
}

impl Default for DesktopServices {
    fn default() -> Self {
        Self {
            login_item: Default::default(),
            power: fastframe_power::Power::fixed(Default::default()),
        }
    }
}
