//! Bar items (0.17: a Omarchy Overlay menu page). Modules are placed by choosing, not typing.
use super::*;
use crate::bar_model::{Module, DISPLAY_MODULES};
/// Every module the bar can show, dividers last.
pub(super) fn modules() -> Vec<Module> {
    DISPLAY_MODULES
        .iter()
        .map(|(m, _)| *m)
        .chain(std::iter::once(Module::Divider))
        .collect()
}
impl App {
    /// 0.17: Bar items is a page of the Omarchy Overlay menu.
    pub(super) unsafe fn show_bar_editor(&mut self) {
        self.open_level(overlay::Level::Page("bar-items"), true);
    }
}
