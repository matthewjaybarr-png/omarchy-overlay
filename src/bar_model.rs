//! Portable bar preferences and collision-free geometry shared by paint and hit testing.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CLOCK_FORMATS: [(&str, &str); 6] = [
    ("time", "Time — 24 hour"),
    ("day-time", "Day + time — 24 hour"),
    ("date-time", "Date + time — 24 hour"),
    ("time-12h", "Time — 12 hour (AM/PM)"),
    ("day-time-12h", "Day + time — 12 hour"),
    ("date-time-12h", "Date + time — 12 hour"),
];
pub fn clock_label(format: &str, date: [u16; 3], weekday: u16, hour: u16, minute: u16) -> String {
    let time = if format.ends_with("-12h") {
        format!(
            "{}:{minute:02} {}",
            if hour.is_multiple_of(12) {
                12
            } else {
                hour % 12
            },
            if hour < 12 { "AM" } else { "PM" }
        )
    } else {
        format!("{hour:02}:{minute:02}")
    };
    match format.strip_suffix("-12h").unwrap_or(format) {
        "time" => time,
        "date-time" => format!("{:04}-{:02}-{:02}  {time}", date[0], date[1], date[2]),
        _ => format!(
            "{} {:02}  {time}",
            ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][weekday as usize % 7],
            date[2]
        ),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Module {
    Start,
    Launcher,
    WorkspaceNav,
    WindowScroll,
    SystemTray,
    Menu,
    Workspaces,
    Tiling,
    Tasks,
    AppDrawer,
    Notifications,
    QuickSettings,
    Display,
    PowerMenu,
    Clipboard,
    Capture,
    Title,
    Clock,
    Sound,
    Network,
    Power,
    Controls,
    Divider,
}
impl Module {
    pub fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Launcher => "launcher",
            Self::WorkspaceNav => "workspace-nav",
            Self::WindowScroll => "window-scroll",
            Self::SystemTray => "system-tray",
            Self::Menu => "menu",
            Self::Workspaces => "workspaces",
            Self::Tiling => "tiling",
            Self::Tasks => "tasks",
            Self::AppDrawer => "app-drawer",
            Self::Notifications => "notifications",
            Self::QuickSettings => "quick-settings",
            Self::Display => "display",
            Self::PowerMenu => "power-menu",
            Self::Clipboard => "clipboard",
            Self::Capture => "capture",
            Self::Title => "title",
            Self::Clock => "clock",
            Self::Sound => "sound",
            Self::Network => "network",
            Self::Power => "power",
            Self::Controls => "controls",
            Self::Divider => "divider",
        }
    }
}
pub const DISPLAY_MODULES: [(Module, &str); 22] = [
    (Module::Start, "Windows Start menu"),
    (Module::Launcher, "Omarchy Overlay app picker"),
    (Module::Menu, "Omarchy Overlay menu"),
    (Module::Workspaces, "Workspace buttons"),
    (Module::WorkspaceNav, "Previous / next desktop"),
    (Module::Tiling, "Tiling status"),
    (Module::Tasks, "Running apps"),
    (Module::Title, "Focused window title"),
    (Module::Clock, "Clock"),
    (Module::Sound, "Sound"),
    (Module::Network, "Network"),
    (Module::Power, "Power"),
    (Module::Controls, "Controls"),
    (Module::AppDrawer, "Open-window drawer"),
    (Module::Notifications, "Windows notifications"),
    (Module::QuickSettings, "Windows Quick Settings"),
    (Module::Display, "Windows display settings"),
    (Module::PowerMenu, "Windows power-user menu"),
    (Module::Clipboard, "Windows clipboard history"),
    (Module::Capture, "Windows screen capture"),
    (Module::WindowScroll, "Scroll tiled windows"),
    (Module::SystemTray, "Windows system tray"),
];
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    #[default]
    Text,
    IconText,
    Icon,
}
impl DisplayMode {
    pub fn icon(self) -> bool {
        self != Self::Text
    }
    pub fn text(self) -> bool {
        self != Self::Icon
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "BarConfigData")]
pub struct BarConfig {
    pub height: i32,
    pub font_size: i32,
    pub font_family: String,
    pub spacing: i32,
    pub task_buttons: usize,
    pub split_task_modules: bool,
    pub module_display: BTreeMap<Module, DisplayMode>,
    pub clock_format: String,
    pub workspace_labels: Vec<String>,
    pub left: Vec<Module>,
    pub center: Vec<Module>,
    pub right: Vec<Module>,
}
#[derive(Deserialize)]
#[serde(default)]
struct BarConfigData {
    height: i32,
    font_size: i32,
    font_family: String,
    spacing: i32,
    task_buttons: usize,
    split_task_modules: bool,
    module_display: BTreeMap<Module, DisplayMode>,
    clock_format: String,
    workspace_labels: Vec<String>,
    left: Vec<Module>,
    center: Vec<Module>,
    right: Vec<Module>,
}
impl Default for BarConfigData {
    fn default() -> Self {
        let c = BarConfig::default();
        Self {
            height: c.height,
            font_size: c.font_size,
            font_family: c.font_family,
            spacing: c.spacing,
            task_buttons: c.task_buttons,
            split_task_modules: false,
            // Profiles saved before icon defaults keep their text labels.
            module_display: BTreeMap::new(),
            clock_format: c.clock_format,
            workspace_labels: c.workspace_labels,
            left: c.left,
            center: c.center,
            right: c.right,
        }
    }
}
impl TryFrom<BarConfigData> for BarConfig {
    type Error = String;
    fn try_from(c: BarConfigData) -> Result<Self, String> {
        let mut result = Self {
            height: c.height,
            font_size: c.font_size,
            font_family: c.font_family,
            spacing: c.spacing,
            task_buttons: c.task_buttons,
            split_task_modules: c.split_task_modules,
            module_display: c.module_display,
            clock_format: c.clock_format,
            workspace_labels: c.workspace_labels,
            left: c.left,
            center: c.center,
            right: c.right,
        };
        if !result.split_task_modules {
            let has_drawer = [&result.left, &result.center, &result.right]
                .iter()
                .any(|s| s.contains(&Module::AppDrawer));
            if !has_drawer {
                for section in [&mut result.left, &mut result.center, &mut result.right] {
                    if let Some(i) = section.iter().position(|m| *m == Module::Tasks) {
                        section.insert(i, Module::AppDrawer);
                        break;
                    }
                }
            }
            let mode = result.display(Module::Tasks);
            result
                .module_display
                .entry(Module::AppDrawer)
                .or_insert(mode);
            result.split_task_modules = true;
        }
        result.validate()?;
        Ok(result)
    }
}
/// Omarchy's own bar glyph (JetBrainsMono Nerd Font) for a module, given its status text.
/// None keeps the drawn line icon; Omarchy's logo lives in its own font, which is not shipped.
pub fn omarchy_glyph(module: Module, active: bool, status: &str) -> Option<char> {
    let battery = |rest: &str, charging: bool| {
        let percent: usize = rest.strip_suffix('%')?.parse().ok()?;
        let i = (percent / 10).min(9);
        let icons = if charging {
            [
                '\u{f089c}',
                '\u{f0086}',
                '\u{f0087}',
                '\u{f0088}',
                '\u{f089d}',
                '\u{f0089}',
                '\u{f089e}',
                '\u{f008a}',
                '\u{f008b}',
                '\u{f0085}',
            ]
        } else {
            [
                '\u{f007a}',
                '\u{f007b}',
                '\u{f007c}',
                '\u{f007d}',
                '\u{f007e}',
                '\u{f007f}',
                '\u{f0080}',
                '\u{f0081}',
                '\u{f0082}',
                '\u{f0079}',
            ]
        };
        Some(icons[i])
    };
    match module {
        Module::SystemTray => Some('\u{f053}'),
        Module::Sound if status.starts_with("MUTE") => Some('\u{f0581}'),
        Module::Sound => Some('\u{f028}'),
        Module::Network if status.starts_with("WI-FI") => Some('\u{f0928}'),
        Module::Network if status.starts_with("ETH") => Some('\u{f0200}'),
        Module::Network => Some('\u{f092e}'),
        Module::Power => {
            let value = status.lines().next().unwrap_or("");
            match value.strip_prefix('+') {
                Some(rest) => battery(rest, true),
                None => battery(value, false),
            }
        }
        Module::Tiling => Some(if active { '\u{f14fb}' } else { '\u{f0536}' }),
        Module::Notifications => Some('\u{f009a}'),
        Module::Clipboard => Some('\u{f014c}'),
        Module::Capture => Some('\u{f0ec2}'),
        _ => None,
    }
}
impl Default for BarConfig {
    fn default() -> Self {
        use Module::*;
        Self {
            height: 30,
            font_size: 14,
            font_family: "JetBrainsMono Nerd Font".into(),
            spacing: 6,
            task_buttons: 2,
            split_task_modules: true,
            // Omarchy's bar is icons except the clock; the window title only makes sense as text.
            module_display: DISPLAY_MODULES
                .iter()
                .filter(|(m, _)| !matches!(m, Clock | Title))
                .map(|&(m, _)| (m, DisplayMode::Icon))
                .collect(),
            clock_format: "day-time".into(),
            workspace_labels: vec!["1".into(), "2".into(), "3".into(), "4".into()],
            // Omarchy's Waybar: logo menu and workspaces, clock, tray and status icons.
            left: vec![Menu, Workspaces],
            center: vec![Clock],
            right: vec![SystemTray, Sound, Network, Power],
        }
    }
}
impl BarConfig {
    pub fn display(&self, module: Module) -> DisplayMode {
        self.module_display
            .get(&module)
            .copied()
            .unwrap_or(if module == Module::Tasks {
                DisplayMode::IconText
            } else {
                DisplayMode::Text
            })
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(24..=56).contains(&self.height)
            || !(10..=22).contains(&self.font_size)
            || self.font_size > self.height - 8
            || !(0..=24).contains(&self.spacing)
        {
            return Err(
                "Bar height must be 24–56, font 10–22 (at least 8 below height), and spacing 0–24."
                    .into(),
            );
        }
        if self.task_buttons > 8 {
            return Err("Choose 0–8 running app buttons per monitor.".into());
        }
        if self.font_family.trim().is_empty()
            || self.font_family.len() > 80
            || self.font_family.chars().any(char::is_control)
        {
            return Err("Choose a valid installed font family (maximum 80 bytes).".into());
        }
        if !CLOCK_FORMATS
            .iter()
            .any(|(name, _)| *name == self.clock_format)
        {
            return Err("Choose a supported 12-hour or 24-hour clock format.".into());
        }
        if self.workspace_labels.is_empty()
            || self.workspace_labels.len() > 16
            || self.workspace_labels.iter().any(|s| {
                s.trim().is_empty()
                    || s.chars().count() > 12
                    || s.chars().any(|c| c.is_control() || c == ',')
            })
        {
            return Err("Provide 1–16 workspace labels, each 1–12 characters.".into());
        }
        if self.module_display.contains_key(&Module::Divider) {
            return Err("Dividers do not have an icon/text mode.".into());
        }
        let mut seen = vec![];
        for section in [&self.left, &self.center, &self.right] {
            if section.len() > 24 {
                return Err("Each bar section supports at most 24 modules.".into());
            }
            for &module in section {
                if module != Module::Divider {
                    if seen.contains(&module) {
                        return Err(format!(
                            "The {} module can appear only once.",
                            module.name()
                        ));
                    }
                    seen.push(module);
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct Item {
    pub module: Module,
    pub label: String,
    pub width: i32,
    pub action: usize,
    pub active: bool,
    pub icon: bool,
    pub tooltip: String,
}
#[derive(Debug)]
pub struct Slot {
    pub item: Item,
    pub left: i32,
    pub right: i32,
}
/// Side groups keep their anchors; the center remains truly centered when space permits.
/// Oversized groups truncate or omit modules rather than overlap adjacent hit targets.
pub fn arrange(width: i32, spacing: i32, sections: [Vec<Item>; 3]) -> Vec<Slot> {
    let padding = 8.min(width.max(0) / 2);
    let available = (width - 2 * padding).max(0);
    let center_w = sections[1].iter().map(|i| i.width).sum::<i32>()
        + spacing * sections[1].len().saturating_sub(1) as i32;
    let center_w = center_w.min(available / 3);
    let side_budget = ((available - center_w - 2 * spacing).max(0) / 2).max(0);
    fn pack(items: Vec<Item>, start: i32, budget: i32, spacing: i32) -> Vec<Slot> {
        let mut out = vec![];
        let mut x = start;
        let end = start + budget;
        for item in items {
            let remain = end - x;
            if remain < 16 {
                break;
            }
            let size = item.width.min(remain);
            if size < item.width && item.module != Module::Title {
                break;
            }
            out.push(Slot {
                item,
                left: x,
                right: x + size,
            });
            x += size + spacing;
        }
        out
    }
    let [left, center, right] = sections;
    let mut l = pack(left, padding, side_budget, spacing);
    let mut r = pack(right, 0, side_budget, spacing);
    let rw = r.last().map_or(0, |s| s.right);
    for s in &mut r {
        s.left += width - padding - rw;
        s.right += width - padding - rw;
    }
    let low = l.last().map_or(padding, |s| s.right + spacing);
    let high = r.first().map_or(width - padding, |s| s.left - spacing);
    let cw = center_w.min((high - low).max(0));
    let start = ((width - cw) / 2).max(low).min((high - cw).max(low));
    let c = pack(center, start, cw, spacing);
    l.extend(c);
    l.extend(r);
    l
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waybar_style_defaults() {
        use Module::*;
        let c = BarConfig::default();
        assert_eq!(c.left, vec![Menu, Workspaces]);
        assert_eq!(c.center, vec![Clock]);
        assert_eq!(c.right, vec![SystemTray, Sound, Network, Power]);
        assert_eq!(
            (c.font_family.as_str(), c.font_size),
            ("JetBrainsMono Nerd Font", 14)
        );
        assert_eq!(c.clock_format, "day-time");
        assert!(c.validate().is_ok());
    }
    fn item(m: Module, w: i32) -> Item {
        Item {
            module: m,
            label: String::new(),
            width: w,
            action: 0,
            active: false,
            icon: false,
            tooltip: String::new(),
        }
    }
    #[test]
    fn new_bars_default_to_icons_except_clock_and_title() {
        let c = BarConfig::default();
        assert_eq!(c.display(Module::Clock), DisplayMode::Text);
        assert_eq!(c.display(Module::Title), DisplayMode::Text);
        for m in [
            Module::Menu,
            Module::Workspaces,
            Module::SystemTray,
            Module::Sound,
            Module::Network,
            Module::Power,
            Module::Tasks,
        ] {
            assert_eq!(c.display(m), DisplayMode::Icon, "{m:?}");
        }
        assert!(c.validate().is_ok());
    }
    #[test]
    fn omarchy_glyphs_follow_status() {
        assert_eq!(
            omarchy_glyph(Module::Sound, false, "MUTE\nClick"),
            Some('\u{f0581}')
        );
        assert_eq!(
            omarchy_glyph(Module::Network, false, "ETH\n"),
            Some('\u{f0200}')
        );
        assert_eq!(
            omarchy_glyph(Module::Power, false, "57%"),
            Some('\u{f007f}')
        );
        assert_eq!(
            omarchy_glyph(Module::Power, false, "+100%"),
            Some('\u{f0085}')
        );
        assert_eq!(omarchy_glyph(Module::Power, false, "AC"), None);
        assert_eq!(omarchy_glyph(Module::Menu, false, ""), None);
    }
    #[test]
    fn old_profiles_keep_text_and_native_task_icons() {
        let config: BarConfig =
            serde_json::from_str(r#"{"left":["menu","workspaces"],"clock_format":"day-time"}"#)
                .unwrap();
        assert_eq!(config.display(Module::Menu), DisplayMode::Text);
        assert_eq!(config.display(Module::Tasks), DisplayMode::IconText);
        assert_eq!(config.left, vec![Module::Menu, Module::Workspaces]);
    }
    #[test]
    fn module_modes_round_trip_independently_and_reject_invalid_values() {
        let mut config = BarConfig::default();
        for (i, (module, _)) in DISPLAY_MODULES.iter().enumerate() {
            config.module_display.insert(
                *module,
                match i % 3 {
                    0 => DisplayMode::Icon,
                    1 => DisplayMode::IconText,
                    _ => DisplayMode::Text,
                },
            );
        }
        config.left = vec![Module::Start, Module::Launcher, Module::WorkspaceNav];
        assert!(config.validate().is_ok());
        let encoded = serde_json::to_string(&config).unwrap();
        let loaded: BarConfig = serde_json::from_str(&encoded).unwrap();
        assert_eq!(config.module_display, loaded.module_display);
        assert!(loaded.display(Module::Start).icon());
        assert!(!loaded.display(Module::Start).text());
        assert!(loaded.display(Module::Launcher).icon() && loaded.display(Module::Launcher).text());
        assert!(
            serde_json::from_str::<BarConfig>(r#"{"module_display":{"clock":"invalid"}}"#).is_err()
        );
        assert!(
            serde_json::from_str::<BarConfig>(r#"{"module_display":{"unknown":"icon"}}"#).is_err()
        );
        config
            .module_display
            .insert(Module::Divider, DisplayMode::Icon);
        assert!(config.validate().is_err());
    }
    #[test]
    fn clock_handles_midnight_noon_and_existing_formats() {
        let date = [2026, 10, 28];
        assert_eq!(clock_label("time-12h", date, 3, 0, 5), "12:05 AM");
        assert_eq!(clock_label("time-12h", date, 3, 12, 0), "12:00 PM");
        assert_eq!(clock_label("time-12h", date, 3, 23, 9), "11:09 PM");
        assert_eq!(
            clock_label("day-time-12h", date, 3, 13, 7),
            "WED 28  1:07 PM"
        );
        assert_eq!(
            clock_label("date-time-12h", date, 3, 9, 4),
            "2026-10-28  9:04 AM"
        );
        assert_eq!(clock_label("day-time", date, 3, 13, 7), "WED 28  13:07");
        for (name, _) in CLOCK_FORMATS {
            let mut config = BarConfig::default();
            config.clock_format = name.into();
            assert!(config.validate().is_ok());
            let loaded: BarConfig =
                serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
            assert_eq!(loaded.clock_format, name);
        }
    }
    #[test]
    fn center_stays_centered_and_hits_do_not_overlap() {
        let s = arrange(
            1280,
            6,
            [
                vec![item(Module::Menu, 40), item(Module::Workspaces, 160)],
                vec![item(Module::Clock, 160)],
                vec![item(Module::Sound, 60)],
            ],
        );
        assert_eq!(s[0].item.label, "");
        assert_eq!(s[0].item.action, 0);
        assert!(!s[0].item.active);
        assert!(!s[0].item.icon);
        assert!(s[0].item.tooltip.is_empty());
        let c = s.iter().find(|s| s.item.module == Module::Clock).unwrap();
        assert_eq!((c.left + c.right) / 2, 640);
        for p in s.windows(2) {
            assert!(p[0].right <= p[1].left);
        }
    }
    #[test]
    fn narrow_bars_never_overlap_or_escape() {
        for width in [0, 16, 64, 220, 500, 1280, 3840] {
            let s = arrange(
                width,
                8,
                [
                    vec![item(Module::Menu, 48), item(Module::Title, 400)],
                    vec![item(Module::Clock, 180)],
                    vec![item(Module::Network, 120), item(Module::Controls, 40)],
                ],
            );
            for i in &s {
                assert!(i.left >= 0 && i.right <= width && i.left < i.right);
            }
            for p in s.windows(2) {
                assert!(p[0].right <= p[1].left);
            }
        }
    }
    #[test]
    fn invalid_preferences_rejected() {
        let mut c = BarConfig::default();
        assert!(c.validate().is_ok());
        c.right.push(Module::Clock);
        assert!(c.validate().is_err());
        c = BarConfig::default();
        c.workspace_labels = vec!["\n".into()];
        assert!(c.validate().is_err());
    }
}

impl Module {
    pub fn title(self) -> &'static str {
        if self == Self::Divider {
            "Divider"
        } else {
            DISPLAY_MODULES
                .iter()
                .find(|(m, _)| *m == self)
                .map_or(self.name(), |(_, s)| s)
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::SystemTray => "Open Windows’ original hidden tray icons and app menus. Icons pinned on the Windows taskbar remain there.",
            Self::AppDrawer => "Open the window drawer without showing running-app buttons.",
            Self::Tasks => "Show running-app buttons. Add the drawer separately if wanted.",
            Self::Launcher => "Search installed apps, files and custom launch entries.",
            Self::Start => "Open the native Windows Start menu.",
            Self::Menu | Self::Controls => "Open Omarchy Overlay controls and native Windows actions.",
            Self::Workspaces => {
                "Switch existing native Windows desktops; all monitors switch together."
            }
            Self::WorkspaceNav => "Previous and next native desktop.",
            Self::WindowScroll => "Previous/next window in the scrolling layout. Wheel over these buttons or running apps to scroll.",
            Self::Tiling => "Enable tiling, or pause and restore original placements.",
            Self::Clock => "Clock and date; opens Windows notifications and calendar.",
            Self::Sound => "Volume and mute; opens native Windows Quick Settings.",
            Self::Network => "Connection status; opens Windows Quick Settings.",
            Self::Power => "Power status and native Windows controls.",
            Self::Title => "Title of the focused window.",
            Self::Divider => "A visual separator; can appear more than once.",
            Self::Notifications => "Open native Windows notifications and calendar.",
            Self::QuickSettings => "Open native audio, network and brightness controls.",
            Self::Display => "Open native Windows display settings.",
            Self::PowerMenu => "Open the native Win+X menu, including Windows power options.",
            Self::Clipboard => {
                "Open Windows clipboard history. Windows controls whether history is enabled."
            }
            Self::Capture => "Open native Windows screen capture; Omarchy Overlay does not save captures.",
        }
    }
}
impl BarConfig {
    pub fn section(&self, section: usize) -> &[Module] {
        match section {
            1 => &self.center,
            2 => &self.right,
            _ => &self.left,
        }
    }
    fn section_mut(&mut self, section: usize) -> &mut Vec<Module> {
        match section {
            1 => &mut self.center,
            2 => &mut self.right,
            _ => &mut self.left,
        }
    }
    pub fn add_module(&mut self, section: usize, module: Module) -> Result<(), String> {
        let mut c = self.clone();
        if module != Module::Divider {
            for s in [&mut c.left, &mut c.center, &mut c.right] {
                s.retain(|m| *m != module);
            }
        }
        c.section_mut(section).push(module);
        c.split_task_modules = true;
        c.validate()?;
        *self = c;
        Ok(())
    }
    pub fn remove_module(&mut self, section: usize, index: usize) {
        let s = self.section_mut(section);
        if index < s.len() {
            s.remove(index);
        }
    }
    pub fn reorder_module(&mut self, section: usize, index: usize, delta: i32) -> usize {
        let s = self.section_mut(section);
        if index >= s.len() {
            return index;
        }
        let next = (index as i32 + delta).clamp(0, s.len() as i32 - 1) as usize;
        s.swap(index, next);
        next
    }
}
#[cfg(test)]
mod editor_tests {
    use super::*;
    #[test]
    fn legacy_tasks_expand_once_and_editor_keeps_modules_independent() {
        let mut raw = serde_json::to_value(BarConfig::default()).unwrap();
        raw.as_object_mut().unwrap().remove("split_task_modules");
        raw["left"] = serde_json::json!(["tasks"]);
        let mut c: BarConfig = serde_json::from_value(raw).unwrap();
        assert_eq!(c.left, vec![Module::AppDrawer, Module::Tasks]);
        let once: BarConfig = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(once.left, c.left);
        c.remove_module(0, 0);
        assert_eq!(c.left, vec![Module::Tasks]);
        c.add_module(2, Module::AppDrawer).unwrap();
        assert_eq!(c.right.last(), Some(&Module::AppDrawer));
        c.add_module(1, Module::Tasks).unwrap();
        assert!(!c.left.contains(&Module::Tasks));
        assert!(c.center.contains(&Module::Tasks));
        c.add_module(1, Module::Divider).unwrap();
        let i = c.center.len() - 1;
        assert_eq!(c.reorder_module(1, i, -1), i - 1);
        c.validate().unwrap();
    }
}

impl BarConfig {
    pub fn show_open_windows(&mut self) {
        use Module::*;
        for module in [AppDrawer, Tasks] {
            if ![&self.left, &self.center, &self.right]
                .iter()
                .any(|section| section.contains(&module))
            {
                self.left.push(module);
            }
        }
        self.task_buttons = self.task_buttons.max(4);
        self.module_display.insert(Tasks, DisplayMode::IconText);
        self.module_display
            .entry(AppDrawer)
            .or_insert(DisplayMode::Icon);
    }
}

#[cfg(test)]
mod expansion_tests {
    use super::*;
    #[test]
    fn open_windows_preset_preserves_choices_and_adds_modules_only_once() {
        let mut bar = BarConfig {
            left: vec![Module::Menu],
            center: vec![],
            right: vec![Module::Clock],
            task_buttons: 0,
            ..Default::default()
        };
        bar.show_open_windows();
        bar.show_open_windows();
        bar.validate().unwrap();
        assert_eq!(bar.left, [Module::Menu, Module::AppDrawer, Module::Tasks]);
        assert_eq!(bar.right, [Module::Clock]);
        assert_eq!(bar.task_buttons, 4);
        assert_eq!(bar.display(Module::Tasks), DisplayMode::IconText);
    }
    #[test]
    fn every_catalog_module_has_a_description_and_icon_choice() {
        for (module, _) in DISPLAY_MODULES {
            assert!(!module.title().is_empty());
            assert!(!module.description().is_empty());
        }
        let mut bar = BarConfig::default();
        bar.module_display
            .insert(Module::WindowScroll, DisplayMode::Icon);
        let text = serde_json::to_string(&bar).unwrap();
        let loaded: BarConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.display(Module::WindowScroll), DisplayMode::Icon);
        assert!(!loaded.section(0).is_empty());
    }
}

/// One look for the whole bar, chosen from the menu's "Icons or text" page.
impl DisplayMode {
    pub const ALL: [DisplayMode; 3] = [DisplayMode::Icon, DisplayMode::IconText, DisplayMode::Text];
    pub fn label(self) -> &'static str {
        match self {
            Self::Icon => "Icon only",
            Self::IconText => "Icon and text",
            Self::Text => "Text only",
        }
    }
    /// The next mode, so Enter on an item steps icon → icon and text → text → icon.
    pub fn next(self) -> Self {
        match self {
            Self::Icon => Self::IconText,
            Self::IconText => Self::Text,
            Self::Text => Self::Icon,
        }
    }
}
impl Module {
    /// The module with this `name()`, for items that have an icon/text mode.
    pub fn from_key(key: &str) -> Option<Module> {
        DISPLAY_MODULES.iter().map(|&(m, _)| m).find(|m| m.name() == key)
    }
}
impl BarConfig {
    /// What `set_all_display(mode)` gives one module. Icons keep Omarchy's look: the
    /// clock and the window title stay text, because an icon alone says nothing there.
    fn styled(mode: DisplayMode, module: Module) -> DisplayMode {
        match (mode, module) {
            (DisplayMode::Icon, Module::Clock | Module::Title) => DisplayMode::Text,
            (DisplayMode::IconText, Module::Title) => DisplayMode::Text,
            (mode, _) => mode,
        }
    }
    /// Shows every bar item as icons, icons and text, or text.
    pub fn set_all_display(&mut self, mode: DisplayMode) {
        self.module_display = DISPLAY_MODULES
            .iter()
            .map(|&(m, _)| (m, Self::styled(mode, m)))
            .collect();
    }
    /// The whole-bar style in use, or None when items were set one by one.
    pub fn all_display(&self) -> Option<DisplayMode> {
        DisplayMode::ALL.into_iter().find(|&mode| {
            DISPLAY_MODULES
                .iter()
                .all(|&(m, _)| self.display(m) == Self::styled(mode, m))
        })
    }
    /// Steps one item to its next display mode.
    pub fn cycle_display(&mut self, module: Module) {
        if module != Module::Divider {
            let next = self.display(module).next();
            self.module_display.insert(module, next);
        }
    }
    /// Puts the default modules back in their default places, keeping every other bar choice.
    pub fn reset_layout(&mut self) {
        let d = BarConfig::default();
        self.left = d.left;
        self.center = d.center;
        self.right = d.right;
        self.split_task_modules = d.split_task_modules;
        self.task_buttons = d.task_buttons;
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;
    #[test]
    fn whole_bar_styles_round_trip_and_icons_match_the_defaults() {
        let mut c = BarConfig::default();
        assert_eq!(c.all_display(), Some(DisplayMode::Icon));
        for mode in DisplayMode::ALL {
            c.set_all_display(mode);
            c.validate().unwrap();
            assert_eq!(c.all_display(), Some(mode), "{mode:?}");
        }
        c.set_all_display(DisplayMode::Icon);
        let d = BarConfig::default();
        assert!(DISPLAY_MODULES.iter().all(|&(m, _)| c.display(m) == d.display(m)));
        c.set_all_display(DisplayMode::Text);
        assert!(DISPLAY_MODULES.iter().all(|&(m, _)| !c.display(m).icon()));
    }
    #[test]
    fn one_item_cycles_and_the_bar_reports_a_mixed_style() {
        let mut c = BarConfig::default();
        c.cycle_display(Module::Sound);
        assert_eq!(c.display(Module::Sound), DisplayMode::IconText);
        assert_eq!(c.all_display(), None);
        c.cycle_display(Module::Sound);
        c.cycle_display(Module::Sound);
        assert_eq!(c.display(Module::Sound), DisplayMode::Icon);
        assert_eq!(c.all_display(), Some(DisplayMode::Icon));
        c.cycle_display(Module::Divider);
        c.validate().unwrap();
    }
    #[test]
    fn every_styled_module_is_found_by_its_name() {
        for (m, _) in DISPLAY_MODULES {
            assert_eq!(Module::from_key(m.name()), Some(m));
        }
        assert_eq!(Module::from_key("divider"), None);
    }
    #[test]
    fn layout_reset_keeps_look_and_clock() {
        let mut c = BarConfig { left: vec![], center: vec![], right: vec![Module::Clock], task_buttons: 7, clock_format: "24h".into(), ..Default::default() };
        c.set_all_display(DisplayMode::Text);
        c.reset_layout();
        let d = BarConfig::default();
        assert_eq!((c.left.clone(), c.center.clone(), c.right.clone()), (d.left, d.center, d.right));
        assert_eq!(c.task_buttons, d.task_buttons);
        assert_eq!(c.all_display(), Some(DisplayMode::Text));
        assert_eq!(c.clock_format, "24h");
    }
}
