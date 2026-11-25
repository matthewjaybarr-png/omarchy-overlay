//! Per-PC layout choices: Windows desktop IDs and monitor identities do not travel.
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub entries: Vec<Entry>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub desktop: String,
    pub monitor: String,
    pub layout: String,
}
pub const LAYOUTS: &[(&str, &str)] = &[
    ("main-stack", "Main + stack"),
    ("columns", "Balanced columns"),
    ("rows", "Rows"),
    ("grid", "Grid"),
    ("dwindle", "Dwindle"),
    ("scrolling", "Scrolling — Niri-inspired"),
    ("centered", "Centered main"),
    ("monocle", "Monocle — one at a time"),
    ("floating", "Floating — windows stay where you put them"),
];
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.entries.len() > 512 {
            return Err("Up to 512 workspace/monitor layout choices are supported.".into());
        }
        for (i, e) in self.entries.iter().enumerate() {
            if (e.desktop != "*"
                && (e.desktop.len() != 32 || !e.desktop.bytes().all(|b| b.is_ascii_hexdigit())))
                || e.monitor.is_empty()
                || e.monitor.len() > 4096
                || e.monitor.chars().any(char::is_control)
                || !LAYOUTS.iter().any(|(name, _)| *name == e.layout)
                || self.entries[..i]
                    .iter()
                    .any(|old| old.desktop == e.desktop && old.monitor == e.monitor)
            {
                return Err("Invalid or duplicate workspace layout choice.".into());
            }
        }
        Ok(())
    }
    pub fn effective<'a>(&'a self, desktop: &str, monitor: &str, default: &'a str) -> &'a str {
        for (d, m) in [
            (desktop, monitor),
            (desktop, "*"),
            ("*", monitor),
            ("*", "*"),
        ] {
            if let Some(e) = self
                .entries
                .iter()
                .find(|e| e.desktop == d && e.monitor == m)
            {
                return &e.layout;
            }
        }
        default
    }
    /// The scope a per-workspace change should write: the workspace's own entry for
    /// this monitor or for all monitors if one exists, otherwise a new all-monitors
    /// entry for the workspace, so other workspaces and the default stay as they are.
    pub fn workspace_scope(&self, desktop: &str, monitor: &str) -> (String, String) {
        let own = |m: &str| self.entries.iter().any(|e| e.desktop == desktop && e.monitor == m);
        let monitor = if own(monitor) { monitor } else { "*" };
        (desktop.to_string(), monitor.to_string())
    }
    pub fn set(
        &mut self,
        desktop: String,
        monitor: String,
        layout: Option<String>,
    ) -> Result<(), String> {
        self.entries
            .retain(|e| e.desktop != desktop || e.monitor != monitor);
        if let Some(layout) = layout {
            self.entries.push(Entry {
                desktop,
                monitor,
                layout,
            });
        }
        self.validate()
    }
}
pub fn desktop_key(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn most_specific_choice_wins_without_reassigning_unknown_hardware() {
        let desktop = desktop_key(&[1; 16]);
        let mut p = Preferences::default();
        p.set("*".into(), "screen-a".into(), Some("columns".into()))
            .unwrap();
        p.set(desktop.clone(), "*".into(), Some("rows".into()))
            .unwrap();
        p.set(desktop.clone(), "screen-a".into(), Some("dwindle".into()))
            .unwrap();
        assert_eq!(p.effective(&desktop, "screen-a", "grid"), "dwindle");
        assert_eq!(p.effective(&desktop, "screen-b", "grid"), "rows");
        assert_eq!(p.effective("other", "screen-a", "grid"), "columns");
        assert_eq!(p.effective("other", "unknown", "grid"), "grid");
        p.set(desktop.clone(), "screen-a".into(), None).unwrap();
        assert_eq!(p.effective(&desktop, "screen-a", "grid"), "rows");
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        restored.validate().unwrap();
    }
    #[test]
    fn workspace_changes_stay_on_that_workspace() {
        let desktop = desktop_key(&[2; 16]);
        let mut p = Preferences::default();
        p.set("*".into(), "*".into(), Some("columns".into())).unwrap();
        // No entry of its own yet: the workspace gets one for all its monitors.
        let (d, m) = p.workspace_scope(&desktop, "screen-a");
        assert_eq!((d.as_str(), m.as_str()), (desktop.as_str(), "*"));
        p.set(d, m, Some("floating".into())).unwrap();
        assert_eq!(p.effective(&desktop, "screen-a", "grid"), "floating");
        assert_eq!(p.effective("other", "screen-a", "grid"), "columns");
        // A monitor-specific entry of its own is the one that changes.
        p.set(desktop.clone(), "screen-b".into(), Some("rows".into())).unwrap();
        let (d, m) = p.workspace_scope(&desktop, "screen-b");
        assert_eq!(m, "screen-b");
        p.set(d, m, Some("dwindle".into())).unwrap();
        assert_eq!(p.effective(&desktop, "screen-b", "grid"), "dwindle");
        assert_eq!(p.effective(&desktop, "screen-a", "grid"), "floating");
    }
}
