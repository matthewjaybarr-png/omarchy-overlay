use super::*;
use crate::workspace_layouts::{self, Preferences, LAYOUTS};
/// One saved choice in words, e.g. "Desktop 2 · DISPLAY1 — Rows".
pub(super) unsafe fn entry_label(entry: &workspace_layouts::Entry) -> String {
    format!(
        "{} · {} — {}",
        desktop_label(&entry.desktop),
        monitor_label(&entry.monitor),
        layout_label(&entry.layout)
    )
}
pub(super) fn desktop_label(key: &str) -> String {
    if key == "*" {
        return "All desktops".into();
    }
    crate::desktops::snapshot()
        .as_ref()
        .and_then(|s| {
            s.ids
                .iter()
                .position(|id| workspace_layouts::desktop_key(id) == key)
        })
        .map(|i| format!("Desktop {}", i + 1))
        .unwrap_or_else(|| "Other saved desktop".into())
}
pub(super) unsafe fn monitor_label(identity: &str) -> String {
    if identity == "*" {
        return "All displays".into();
    }
    settings::displays()
        .iter()
        .find(|d| monitor_identity(&d.device) == identity)
        .map(|d| d.device.trim_start_matches("\\\\.\\").to_owned())
        .unwrap_or_else(|| "Disconnected display".into())
}
pub(super) fn layout_label(name: &str) -> &str {
    LAYOUTS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(name, |(_, label)| label)
}
pub(super) fn current_desktop() -> Option<String> {
    crate::desktops::snapshot().map(|s| workspace_layouts::desktop_key(&s.ids[s.current]))
}
pub(super) unsafe fn monitor_identity(device: &str) -> String {
    let mut display: DISPLAY_DEVICEW = zeroed();
    display.cb = size_of::<DISPLAY_DEVICEW>() as u32;
    if EnumDisplayDevicesW(
        wide(device).as_ptr(),
        0,
        &mut display,
        EDD_GET_DEVICE_INTERFACE_NAME,
    ) != 0
    {
        let n = display
            .DeviceID
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(display.DeviceID.len());
        if n > 0 {
            return String::from_utf16_lossy(&display.DeviceID[..n]).to_lowercase();
        }
    }
    device.to_lowercase()
}
impl App {
    /// Display profiles: when the connected monitors change, the layout, main share and gap
    /// last used with that set come back.
    pub(super) unsafe fn check_display_set(&mut self) {
        let mut devices: Vec<String> = settings::displays().into_iter().map(|d| d.device).collect();
        devices.sort();
        let devices = devices.join("|");
        if devices == self.display_devices {
            return;
        }
        let mut names: Vec<String> = devices.split('|').map(|d| monitor_identity(d)).collect();
        names.sort();
        let set = names.join("+");
        self.display_devices = devices;
        self.display_set = Some(set.clone());
        let mut c = self.config.clone();
        if c.recall_display(&set) && self.commit(c) {
            self.show_feedback("Layout for these monitors is back");
        }
    }
    /// The workspace key a monitor's layout choice is stored under: the workspace that
    /// monitor shows (its own one with per-monitor workspaces).
    fn desktop_for(&self, monitor: HMONITOR) -> Option<String> {
        if self.config.per_monitor_workspaces {
            let index = self.workspace_of(monitor as isize);
            return Some(workspace_layouts::desktop_key(&crate::desktops::workspace_id(index)));
        }
        current_desktop()
    }
    pub(super) unsafe fn layout_for_monitor(&self, monitor: HMONITOR) -> String {
        let Some(desktop) = self.desktop_for(monitor) else {
            return self.config.layout.clone();
        };
        let mut info: MONITORINFOEXW = zeroed();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) == 0 {
            return self.config.layout.clone();
        }
        let n = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        let identity = monitor_identity(&String::from_utf16_lossy(&info.szDevice[..n]));
        self.layout_preferences
            .effective(&desktop, &identity, &self.config.layout)
            .into()
    }
    pub(super) unsafe fn layout_scope(&self, monitor: HMONITOR) -> Option<(String, String)> {
        let desktop = self.desktop_for(monitor)?;
        let display = settings::displays()
            .into_iter()
            .find(|d| d.monitor == monitor)?;
        Some((desktop, monitor_identity(&display.device)))
    }
    pub(super) unsafe fn save_layout_choices(
        &mut self,
        choices: Preferences,
    ) -> Result<(), String> {
        choices.validate()?;
        write_json_atomic(&self.directory.join("workspace-layouts.json"), &choices)?;
        self.layout_preferences = choices;
        self.dirty = true;
        self.tick();
        self.update_dashboard();
        Ok(())
    }
    /// 0.17: Layouts by workspace is a page of the Omarchy Overlay menu.
    pub(super) unsafe fn show_workspace_layouts(&mut self) {
        self.layout_all_desktops = false;
        self.layout_monitor = "*".into();
        self.open_level(overlay::Level::Page("workspace-layouts"), true);
    }
    /// The scope the page is editing: (desktop key or "*", display identity or "*").
    pub(super) fn layout_page_scope(&self) -> Option<(String, String)> {
        let desktop = if self.layout_all_desktops {
            "*".into()
        } else {
            current_desktop()?
        };
        Some((desktop, self.layout_monitor.clone()))
    }
    /// Steps the display scope: All displays, then each connected display.
    pub(super) unsafe fn next_layout_monitor(&mut self) {
        let mut list = vec!["*".to_string()];
        list.extend(
            settings::displays()
                .iter()
                .map(|d| monitor_identity(&d.device)),
        );
        let i = list
            .iter()
            .position(|m| *m == self.layout_monitor)
            .map_or(0, |i| i + 1);
        self.layout_monitor = list[i % list.len()].clone();
    }
}
