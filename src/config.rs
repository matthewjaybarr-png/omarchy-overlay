use crate::rules::AppRule;
use serde::{Deserialize, Serialize};

/// What one set of monitors was last used with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisplaySetup {
    pub layout: String,
    pub main_ratio: f64,
    pub gap: i32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Display profiles: keep the layout, main share and gap of each set of monitors.
    pub remember_displays: bool,
    /// Keyed by the monitors connected together, see `remember_display`.
    pub display_setups: std::collections::BTreeMap<String, DisplaySetup>,
    pub bar: crate::bar_model::BarConfig,
    pub gap: i32,
    /// Space between tiled windows and the monitor edge (Hyprland gaps_out).
    pub outer_gap: i32,
    /// Square corners and a 1 px theme border on managed windows, accent when focused.
    pub window_borders: bool,
    pub main_ratio: f64,
    pub modifiers: String,
    pub cycle_workspace_shortcut: String,
    pub shortcut_overrides: std::collections::BTreeMap<String, String>,
    pub applications: Vec<crate::portable_apps::Application>,
    pub terminal: String,
    pub excluded_processes: Vec<String>,
    pub show_top_bar: bool,
    pub auto_hide_taskbar: bool,
    pub theme: String,
    pub focus_outline: bool,
    pub show_osd: bool,
    pub appearance_profiles: Vec<crate::appearance_profiles::Profile>,
    pub appearance_wallpaper: String,
    pub bar_opacity: u8,
    pub inactive_opacity: u8,
    pub layout: String,
    pub scrolling_columns: usize,
    pub rules: Vec<AppRule>,
    pub default_mode: crate::rules::Mode,
    pub default_allow_opacity: bool,
    pub active_opacity: u8,
    pub coordinate_windows: bool,
    /// Window corners: default, square, round or small.
    pub corners: String,
    /// Material for other programs' windows (the default; an app's own rule wins).
    pub window_backdrop: String,
    /// Material for Omarchy Overlay's own menu and app picker. Separate from window_backdrop
    /// since 0.17, so picking a blur for apps never makes Omarchy Overlay unreadable.
    pub menu_backdrop: String,
    /// Turn tiling on at every start (sign-in, Restart Omarchy Overlay, after an update).
    pub start_tiling: bool,
    pub animations: bool,
    pub animation_duration_ms: u32,
    pub taskbar_mode: String,
    /// When the dock shows: "floating", "always" or "off" (dock::MODES).
    pub dock: String,
    /// The themed Windows Terminal profile and VS Code theme are wanted: exit takes them
    /// away like every other change to Windows, and the next start puts them back.
    pub terminal_colors: bool,
    pub vscode_colors: bool,
    /// "omarchy" (Super keys), "classic" (Ctrl+Alt keys) or "both".
    pub keymap: String,
    /// What a tap of Super alone does: "start", "picker" or "nothing".
    pub tap_super: String,
    /// The menu, app picker and keybinds text font. Any installed font is allowed; a font
    /// that is missing on this PC falls back at drawing time. Icons always come from the
    /// Nerd Font (fonts::ICON_FAMILY).
    pub menu_font: String,
    pub menu_font_size: i32,
    /// Loaded from config\always-float.json; never stored in the profile.
    #[serde(skip)]
    pub always_float: crate::always_float::List,
}
pub const KEYMAPS: &[(&str, &str)] = &[
    ("both", "Super keys and Ctrl+Alt keys"),
    ("omarchy", "Super keys only (Omarchy laptop)"),
    ("classic", "Ctrl+Alt keys only (0.15 and earlier)"),
];
pub const TAP_SUPER: &[(&str, &str)] = &[
    ("start", "Open Windows Start"),
    ("picker", "Open the Omarchy Overlay app picker"),
    ("nothing", "Do nothing"),
];
/// Fonts the menu lists first: the bundled ones, then fonts every Windows 11 PC has.
/// The font page adds every other font installed on the PC after these.
pub const MENU_FONTS: &[(&str, &str)] = &[
    (
        "JetBrains Mono NL",
        "JetBrains Mono NL (bundled, no ligatures)",
    ),
    (
        "JetBrainsMono Nerd Font",
        "JetBrainsMono Nerd Font (bundled)",
    ),
    ("Cascadia Mono", "Cascadia Mono (Windows)"),
    ("Consolas", "Consolas (Windows)"),
    ("Segoe UI", "Segoe UI (Windows)"),
];
pub const MATERIALS: [&str; 3] = ["solid", "mica", "acrylic"];
/// Omarchy's window transparency modes (omarchy-hyprland-window-transparency) as
/// (id, label, active opacity %, inactive opacity %, app window material).
pub const TRANSPARENCY: &[(&str, &str, u8, u8, &str)] = &[
    ("opaque", "Opaque windows", 100, 100, "solid"),
    ("transparent", "Transparent windows (Omarchy's default: barely see-through)", 98, 95, "solid"),
    ("glassy", "Glassy windows (75% and Acrylic)", 75, 75, "acrylic"),
];
impl Config {
    /// Sets one of the TRANSPARENCY modes; false for an unknown id.
    pub fn set_transparency(&mut self, id: &str) -> bool {
        let Some(&(_, _, active, inactive, material)) = TRANSPARENCY.iter().find(|m| m.0 == id) else {
            return false;
        };
        self.active_opacity = active;
        self.inactive_opacity = inactive;
        self.window_backdrop = material.into();
        true
    }
    /// The TRANSPARENCY mode these settings match, if any.
    pub fn transparency(&self) -> Option<&'static str> {
        TRANSPARENCY
            .iter()
            .find(|&&(_, _, active, inactive, material)| {
                self.active_opacity == active && self.inactive_opacity == inactive && self.window_backdrop == material
            })
            .map(|m| m.0)
    }
}
/// A menu font name must be a plausible Windows family name; whether it is installed
/// is checked when drawing, so a missing font never makes the profile invalid.
pub fn valid_font_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 64 && !name.chars().any(char::is_control)
}
impl Default for Config {
    fn default() -> Self {
        Self {
            remember_displays: true,
            display_setups: Default::default(),
            bar: crate::bar_model::BarConfig::default(),
            gap: 5,
            outer_gap: 10,
            window_borders: true,
            main_ratio: 0.55,
            modifiers: "CtrlAlt".into(),
            cycle_workspace_shortcut: crate::shortcuts::DEFAULT_CYCLE.into(),
            shortcut_overrides: Default::default(),
            applications: vec![],
            terminal: "wt.exe".into(),
            show_top_bar: true,
            auto_hide_taskbar: true,
            // Omarchy's own default theme.
            theme: "tokyo-night".into(),
            focus_outline: false,
            show_osd: true,
            appearance_profiles: vec![],
            appearance_wallpaper: String::new(),
            bar_opacity: 79,
            inactive_opacity: 100,
            layout: "main-stack".into(),
            scrolling_columns: 2,
            rules: vec![],
            default_mode: crate::rules::Mode::Tile,
            default_allow_opacity: true,
            active_opacity: 100,
            coordinate_windows: false,
            corners: "default".into(),
            window_backdrop: "solid".into(),
            menu_backdrop: "solid".into(),
            start_tiling: true,
            animations: false,
            animation_duration_ms: 140,
            taskbar_mode: "legacy".into(),
            dock: "floating".into(),
            terminal_colors: false,
            vscode_colors: false,
            keymap: "both".into(),
            tap_super: "start".into(),
            menu_font: MENU_FONTS[1].0.into(),
            menu_font_size: 14,
            always_float: Default::default(),
            excluded_processes: vec![
                "Taskmgr.exe".into(),
                "mstsc.exe".into(),
                "msrdc.exe".into(),
                "ApplicationFrameHost.exe".into(),
                "vmconnect.exe".into(),
            ],
        }
    }
}

impl Config {
    /// Writes the current layout, main share and gap down for this set of monitors.
    pub fn remember_display(&mut self, set: &str) {
        if !self.remember_displays {
            return;
        }
        // The oldest name goes first when a 33rd set of monitors turns up.
        while self.display_setups.len() >= 32 && !self.display_setups.contains_key(set) {
            self.display_setups.pop_first();
        }
        self.display_setups.insert(set.into(), self.display_setup());
    }
    fn display_setup(&self) -> DisplaySetup {
        DisplaySetup {
            layout: self.layout.clone(),
            main_ratio: self.main_ratio,
            gap: self.gap,
        }
    }
    /// Takes back what this set of monitors was last used with. False when nothing changes.
    pub fn recall_display(&mut self, set: &str) -> bool {
        let Some(setup) = self.display_setups.get(set).cloned() else {
            return false;
        };
        if !self.remember_displays || setup == self.display_setup() {
            return false;
        }
        (self.layout, self.main_ratio, self.gap) = (setup.layout, setup.main_ratio, setup.gap);
        true
    }

    pub fn validate(&self) -> Result<(), String> {
        self.bar.validate()?;
        if !crate::commands::CORNERS.iter().any(|(name, _)| *name == self.corners) {
            return Err("Unsupported window corners.".into());
        }
        if self.display_setups.len() > 32 || self.display_setups.keys().any(|k| k.len() > 4096) {
            return Err("display_setups holds too many sets of monitors.".into());
        }
        if !(50..=100).contains(&self.active_opacity)
            || !(50..=400).contains(&self.animation_duration_ms)
            || !MATERIALS.contains(&self.window_backdrop.as_str())
            || !MATERIALS.contains(&self.menu_backdrop.as_str())
            || !["legacy", "visible", "auto-hide", "hidden"].contains(&self.taskbar_mode.as_str())
            || !crate::dock::MODES.iter().any(|(m, _)| *m == self.dock)
        {
            return Err("Invalid global appearance, motion or taskbar setting.".into());
        }
        if self.appearance_wallpaper.len() > 4096
            || self.appearance_wallpaper.chars().any(char::is_control)
        {
            return Err("Invalid appearance wallpaper path.".into());
        }
        if self.appearance_profiles.len() > 64 {
            return Err("Up to 64 appearance profiles are supported.".into());
        }
        for (i, p) in self.appearance_profiles.iter().enumerate() {
            p.validate()?;
            if self.appearance_profiles[..i]
                .iter()
                .any(|old| old.name.eq_ignore_ascii_case(&p.name))
            {
                return Err("Duplicate appearance profile name.".into());
            }
        }

        crate::shortcuts::parse(&self.cycle_workspace_shortcut)?;
        crate::shortcuts::validate_overrides(&self.shortcut_overrides)?;
        crate::portable_apps::validate(&self.applications)?;
        if !self.main_ratio.is_finite() || !(0.25..=0.75).contains(&self.main_ratio) {
            return Err("main_ratio must be between 0.25 and 0.75.".into());
        }
        if !(0..=80).contains(&self.gap) || !(0..=80).contains(&self.outer_gap) {
            return Err("gap and outer_gap must be between 0 and 80.".into());
        }
        if !["WinAlt", "CtrlAlt", "Alt"].contains(&self.modifiers.as_str()) {
            return Err("Unsupported shortcut modifier.".into());
        }
        // Theme files in config\themes can add names, so only the spelling is checked here.
        if !crate::theme_files::valid_name(&self.theme) {
            return Err("Unsupported theme.".into());
        }
        if !KEYMAPS.iter().any(|(k, _)| *k == self.keymap)
            || !TAP_SUPER.iter().any(|(k, _)| *k == self.tap_super)
        {
            return Err("Unsupported keymap or Super key setting.".into());
        }
        if !valid_font_name(&self.menu_font) || !(9..=18).contains(&self.menu_font_size) {
            return Err("Unsupported menu font or size.".into());
        }
        if !(50..=100).contains(&self.bar_opacity) || !(70..=100).contains(&self.inactive_opacity) {
            return Err("Opacity settings are outside the supported range.".into());
        }
        if !(1..=4).contains(&self.scrolling_columns) {
            return Err("Scrolling columns must be between 1 and 4.".into());
        }
        if !crate::commands::LAYOUTS.iter().any(|(name, _)| *name == self.layout) {
            return Err("Unsupported layout.".into());
        }
        if self.rules.len() > 256
            || self.rules.iter().any(|r| {
                !r.valid()
                    || r.executable.len() > 255
                    || r.title_contains.len() > 1024
                    || r.window_class.len() > 255
            })
        {
            return Err("Invalid app rules (maximum 256 rules).".into());
        }
        if self.terminal.len() > 4096 || self.excluded_processes.len() > 256 {
            return Err("Profile exceeds supported limits.".into());
        }
        Ok(())
    }
}

impl Config {
    /// Every setting back to its default. Saved profiles stay, so a reset is never the
    /// way to lose them, and the always-float list stays because it lives in its own file.
    pub fn reset_keeping_profiles(&self) -> Config {
        Config {
            appearance_profiles: self.appearance_profiles.clone(),
            always_float: self.always_float.clone(),
            ..Config::default()
        }
    }
    pub fn policy(&self, exe: &str, title: &str, class: &str) -> crate::rules::Policy {
        if self.always_float.matches(exe, title, class).is_some() {
            return crate::rules::Policy {
                mode: crate::rules::Mode::Float,
                ..self.policy_from_rules(exe, title, class)
            };
        }
        self.policy_from_rules(exe, title, class)
    }
    fn policy_from_rules(&self, exe: &str, title: &str, class: &str) -> crate::rules::Policy {
        let rule = crate::rules::matching(&self.rules, exe, title, class);
        crate::rules::Policy {
            mode: rule.map_or(self.default_mode, |r| r.mode),
            allow_opacity: rule.map_or(self.default_allow_opacity, |r| r.allow_opacity),
            active_opacity: rule
                .and_then(|r| r.active_opacity)
                .unwrap_or(self.active_opacity),
            inactive_opacity: rule
                .and_then(|r| r.inactive_opacity)
                .unwrap_or(self.inactive_opacity),
            material_explicit: rule
                .and_then(|r| r.backdrop.as_deref())
                .is_some_and(|s| s == "mica" || s == "acrylic"),
            backdrop: match rule
                .and_then(|r| r.backdrop.as_deref())
                .unwrap_or(&self.window_backdrop)
            {
                "mica" => 2,
                "acrylic" => 3,
                _ => 0,
            },
            coordinate_theme: rule
                .and_then(|r| r.coordinate_theme)
                .unwrap_or(self.coordinate_windows),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparency_modes_set_and_read_back() {
        let mut c = Config::default();
        assert_eq!(c.transparency(), Some("opaque"));
        for (id, ..) in TRANSPARENCY {
            assert!(c.set_transparency(id));
            c.validate().unwrap();
            assert_eq!(c.transparency(), Some(*id));
        }
        assert!(!c.set_transparency("frosted"));
        c.active_opacity = 90;
        assert_eq!(c.transparency(), None);
    }
    #[test]
    fn each_set_of_monitors_gets_its_layout_back() {
        let mut c = Config::default();
        c.remember_display("desk");
        c.layout = "monocle".into();
        c.gap = 2;
        c.remember_display("laptop");
        assert!(c.recall_display("desk") && c.layout == Config::default().layout);
        assert!(c.recall_display("laptop") && c.layout == "monocle" && c.gap == 2);
        assert!(!c.recall_display("laptop") && !c.recall_display("never seen"));
        c.remember_displays = false;
        assert!(!c.recall_display("desk") && c.layout == "monocle");
        for n in 0..40 {
            c.remember_displays = true;
            c.remember_display(&format!("set {n:02}"));
        }
        assert!(c.display_setups.len() == 32 && c.validate().is_ok());
    }
    #[test]
    fn reset_restores_defaults_but_keeps_saved_profiles() {
        let mut c = Config {
            theme: "gruvbox".into(),
            gap: 20,
            ..Config::default()
        };
        c.bar.set_all_display(crate::bar_model::DisplayMode::Text);
        c.appearance_profiles = crate::appearance_profiles::builtins()[..1].to_vec();
        let reset = c.reset_keeping_profiles();
        let d = Config::default();
        assert_eq!((reset.theme.as_str(), reset.gap), (d.theme.as_str(), d.gap));
        assert_eq!(reset.bar.all_display(), Some(crate::bar_model::DisplayMode::Icon));
        assert_eq!(reset.appearance_profiles.len(), 1);
        reset.validate().unwrap();
    }
    #[test]
    fn walker_menu_font_by_default() {
        let c = Config::default();
        assert_eq!((c.menu_font.as_str(), c.menu_font_size), ("JetBrainsMono Nerd Font", 14));
    }
    #[test]
    fn waybar_bar_opacity_by_default() {
        assert_eq!(Config::default().bar_opacity, 79);
    }
    #[test]
    fn hyprland_gaps_and_borders_by_default_and_for_older_profiles() {
        let c = Config::default();
        assert_eq!((c.gap, c.outer_gap, c.window_borders), (5, 10, true));
        let old: Config = serde_json::from_str(r#"{"theme":"tokyo-night"}"#).unwrap();
        assert_eq!((old.gap, old.outer_gap, old.window_borders), (5, 10, true));
        let wide = Config {
            outer_gap: 81,
            ..Default::default()
        };
        assert!(wide.validate().is_err());
    }
    #[test]
    fn tiling_starts_on_by_default_and_for_older_settings() {
        assert!(Config::default().start_tiling);
        // A 0.16 file has no start_tiling key; K7 wants it to tile after the update too.
        let old: Config = serde_json::from_str(r#"{"layout":"columns"}"#).unwrap();
        assert!(old.start_tiling);
        let off: Config = serde_json::from_str(r#"{"start_tiling":false}"#).unwrap();
        assert!(!off.start_tiling);
    }
    #[test]
    fn an_app_tile_rule_survives_save_and_load() {
        let mut c = Config::default();
        c.rules.push(AppRule {
            executable: "Paint.exe".into(),
            mode: crate::rules::Mode::Tile,
            ..Default::default()
        });
        let c: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        c.validate().unwrap();
        assert_eq!(c.rules[0].mode, crate::rules::Mode::Tile);
        assert_eq!(c.policy("Paint.exe", "", "").mode, crate::rules::Mode::Tile);
    }
    #[test]
    fn global_defaults_and_explicit_app_choices_are_independent() {
        let mut c = Config {
            default_mode: crate::rules::Mode::Float,
            active_opacity: 95,
            ..Default::default()
        };
        c.rules.push(AppRule {
            executable: "OUTLOOK.EXE".into(),
            mode: crate::rules::Mode::Tile,
            active_opacity: Some(90),
            ..Default::default()
        });
        assert_eq!(
            c.policy("olk.exe", "Mail", "MailWindow").mode,
            crate::rules::Mode::Float
        );
        let p = c.policy("OUTLOOK.exe", "Mail", "ClassicWindow");
        assert_eq!(p.mode, crate::rules::Mode::Tile);
        assert_eq!(p.active_opacity, 90);
        assert!(p.allow_opacity);
        assert_eq!(p.inactive_opacity, 100);
        assert!(!p.coordinate_theme);
        assert_eq!(p.mode.label(), "Tile");
        assert_eq!(crate::rules::app_name("olk.exe"), "Outlook (new)");
        assert_eq!(c.policy("other.exe", "Other", "").active_opacity, 95);
        let restored: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(
            restored.policy("olk.exe", "", "").mode,
            crate::rules::Mode::Float
        );
    }
    #[test]
    fn always_float_list_wins_over_app_rules() {
        let mut c = Config::default();
        c.rules.push(AppRule {
            executable: "explorer.exe".into(),
            mode: crate::rules::Mode::Tile,
            ..Default::default()
        });
        assert_eq!(c.theme, "tokyo-night");
        assert_eq!(
            c.policy("explorer.exe", "Copying", "OperationStatusWindow")
                .mode,
            crate::rules::Mode::Float
        );
        assert_eq!(
            c.policy("explorer.exe", "Documents", "CabinetWClass").mode,
            crate::rules::Mode::Tile
        );
        c.always_float.disabled.push("file-operations".into());
        assert_eq!(
            c.policy("explorer.exe", "Copying", "OperationStatusWindow")
                .mode,
            crate::rules::Mode::Tile
        );
        c.validate().unwrap();
        c.keymap = "both-ways".into();
        assert!(c.validate().is_err());
    }
}
