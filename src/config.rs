use crate::rules::AppRule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
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
/// A menu font name must be a plausible Windows family name; whether it is installed
/// is checked when drawing, so a missing font never makes the profile invalid.
pub fn valid_font_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 64 && !name.chars().any(char::is_control)
}
impl Default for Config {
    fn default() -> Self {
        Self {
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
            theme: "neon-ridge".into(),
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
            window_backdrop: "solid".into(),
            menu_backdrop: "solid".into(),
            start_tiling: true,
            animations: false,
            animation_duration_ms: 140,
            taskbar_mode: "legacy".into(),
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
    pub fn validate(&self) -> Result<(), String> {
        self.bar.validate()?;
        if !(50..=100).contains(&self.active_opacity)
            || !(50..=400).contains(&self.animation_duration_ms)
            || !MATERIALS.contains(&self.window_backdrop.as_str())
            || !MATERIALS.contains(&self.menu_backdrop.as_str())
            || !["legacy", "visible", "auto-hide", "hidden"].contains(&self.taskbar_mode.as_str())
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
        if ![
            "main-stack",
            "columns",
            "rows",
            "grid",
            "dwindle",
            "scrolling",
        ]
        .contains(&self.layout.as_str())
        {
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
        assert_eq!(c.theme, "neon-ridge");
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
