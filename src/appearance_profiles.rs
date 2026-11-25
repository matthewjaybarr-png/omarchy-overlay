use crate::config::Config;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub theme: String,
    pub gap: i32,
    pub font_family: String,
    pub font_size: i32,
    pub bar_height: i32,
    pub bar_opacity: u8,
    pub focus_outline: bool,
    pub show_osd: bool,
    #[serde(default)]
    pub wallpaper: String,
    #[serde(default)]
    pub advanced: Option<Advanced>,
}
pub const THEMES: &[(&str, &str)] = &[
    ("neon-ridge", "Neon Ridge"),
    ("tokyo-night", "Tokyo Night"),
    ("everforest", "Everforest"),
    ("nord", "Nord"),
    ("flexoki-light", "Flexoki Light"),
    ("alpenglow-night", "Alpenglow Night"),
];
/// Menu commands 20-24, in id order.
pub const COMMAND_THEMES: [&str; 5] = ["everforest", "tokyo-night", "nord", "flexoki-light", "alpenglow-night"];
impl Profile {
    pub fn capture(config: &Config, name: String) -> Self {
        Self {
            name,
            theme: config.theme.clone(),
            gap: config.gap,
            font_family: config.bar.font_family.clone(),
            font_size: config.bar.font_size,
            bar_height: config.bar.height,
            bar_opacity: config.bar_opacity,
            focus_outline: config.focus_outline,
            show_osd: config.show_osd,
            wallpaper: config.appearance_wallpaper.clone(),
            advanced: Some(Advanced::capture(config)),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.wallpaper.len() > 4096 || self.wallpaper.chars().any(char::is_control) {
            return Err("Invalid wallpaper path.".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.name.chars().any(char::is_control)
            || !crate::theme_files::valid_name(&self.theme)
            || !(0..=80).contains(&self.gap)
            || !(50..=100).contains(&self.bar_opacity)
        {
            return Err("Enter a profile name, valid theme, gaps 0–80 and opacity 50–100.".into());
        }
        if let Some(a) = &self.advanced {
            a.validate()?;
        }
        let bar = crate::bar_model::BarConfig {
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            height: self.bar_height,
            ..Default::default()
        };
        bar.validate()
    }
    pub fn apply(&self, config: &mut Config) -> Result<(), String> {
        self.validate()?;
        config.theme = self.theme.clone();
        config.gap = self.gap;
        config.bar.font_family = self.font_family.clone();
        config.bar.font_size = self.font_size;
        config.bar.height = self.bar_height;
        config.bar_opacity = self.bar_opacity;
        config.focus_outline = self.focus_outline;
        config.show_osd = self.show_osd;
        config.appearance_wallpaper = self.wallpaper.clone();
        if let Some(a) = &self.advanced {
            a.apply(config);
        }
        Ok(())
    }
}
pub fn builtins() -> Vec<Profile> {
    THEMES
        .iter()
        .map(|(theme, name)| Profile {
            name: (*name).into(),
            theme: (*theme).into(),
            gap: if *theme == "neon-ridge" { 4 } else { 8 },
            font_family: if *theme == "neon-ridge" {
                "JetBrains Mono NL"
            } else {
                "Consolas"
            }
            .into(),
            font_size: if *theme == "neon-ridge" { 12 } else { 13 },
            bar_height: if *theme == "neon-ridge" { 24 } else { 30 },
            bar_opacity: 100,
            focus_outline: true,
            show_osd: true,
            wallpaper: if *theme == "neon-ridge" {
                "wallpapers/neon-ridge.jpg"
            } else {
                ""
            }
            .into(),
            advanced: None,
        })
        .collect()
}
/// Windows Terminal scheme keys and the Omarchy colors.toml keys that fill them, in
/// order of preference.
const TERMINAL_COLORS: &[(&str, &[&str])] = &[
    // As Omarchy's terminal templates: black is the background, white the foreground,
    // bright black the muted color and bright white the bright foreground.
    ("black", &["background", "black"]),
    ("red", &["red"]),
    ("green", &["green"]),
    ("yellow", &["yellow"]),
    ("blue", &["blue"]),
    ("purple", &["magenta"]),
    ("cyan", &["cyan"]),
    ("white", &["foreground", "white"]),
    ("brightBlack", &["muted", "bright_black"]),
    ("brightRed", &["bright_red"]),
    ("brightGreen", &["bright_green"]),
    ("brightYellow", &["bright_yellow"]),
    ("brightBlue", &["bright_blue"]),
    ("brightPurple", &["bright_magenta"]),
    ("brightCyan", &["bright_cyan"]),
    ("brightWhite", &["bright_foreground", "bright_white"]),
    ("cursorColor", &["cursor"]),
    ("selectionBackground", &["selection_background", "selection"]),
];
/// The Terminal fragment for a theme. `colors` holds the theme's colors.toml values
/// (0xrrggbb by key); every terminal color the theme defines replaces the default.
pub fn terminal_fragment(
    theme: &str,
    background: &str,
    foreground: &str,
    accent: &str,
    colors: &std::collections::BTreeMap<String, u32>,
    font: Option<&str>,
) -> serde_json::Value {
    let name = format!("Omarchy Overlay {theme}");
    let mut fragment = serde_json::json!({"profiles":[{"name":"Omarchy Overlay","commandline":"powershell.exe","colorScheme":name}],
        "schemes":[{"name":name,"background":background,"foreground":foreground,"cursorColor":accent,"selectionBackground":accent,
            "black":"#1A1B26","red":"#F7768E","green":"#9ECE6A","yellow":"#E0AF68","blue":accent,"purple":"#BB9AF7","cyan":"#7DCFFF","white":foreground,
            "brightBlack":"#565F89","brightRed":"#FF9EAD","brightGreen":"#B9F27C","brightYellow":"#FFD694","brightBlue":accent,"brightPurple":"#D2B1FF","brightCyan":"#A4E4FF","brightWhite":"#FFFFFF"}]});
    // Omarchy's fallbacks fill keys a theme leaves out (bright colors, muted).
    let colors = crate::vscode_theme::resolved(colors);
    for (key, sources) in TERMINAL_COLORS {
        if let Some(value) = sources.iter().find_map(|k| colors.get(*k)) {
            fragment["schemes"][0][key] = serde_json::json!(format!("#{value:06X}"));
        }
    }
    if let Some(face) = font {
        fragment["profiles"][0]["font"] = serde_json::json!({ "face": face });
    }
    if theme == "neon-ridge" {
        for (key, value) in [
            ("black", "#0a1028"),
            ("red", "#f2668f"),
            ("green", "#4fe0b0"),
            ("yellow", "#f5c98a"),
            ("blue", "#5b95fa"),
            ("purple", "#b580fa"),
            ("cyan", "#43d5e8"),
            ("white", "#d7dcf5"),
            ("brightBlack", "#8089bd"),
            ("brightRed", "#ff8fae"),
            ("brightGreen", "#82f0cb"),
            ("brightYellow", "#fbdcae"),
            ("brightBlue", "#8bb5ff"),
            ("brightPurple", "#cfa8ff"),
            ("brightCyan", "#7fe6f3"),
            ("brightWhite", "#f0f2ff"),
            ("selectionBackground", "#2a3a78"),
        ] {
            fragment["schemes"][0][key] = serde_json::json!(value);
        }
    }
    fragment
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_round_trip_and_only_change_appearance_fields() {
        let mut c = Config::default();
        assert_eq!(Profile::capture(&c, "Current".into()).theme, c.theme);
        c.layout = "dwindle".into();
        c.terminal = "my-terminal.exe".into();
        for p in builtins() {
            p.validate().unwrap();
            let p: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
            p.apply(&mut c).unwrap();
            assert_eq!(c.layout, "dwindle");
            assert_eq!(c.terminal, "my-terminal.exe");
        }
        let f = terminal_fragment("Tokyo Night", "#1A1B26", "#C0CAF5", "#7AA2F7", &Default::default(), None);
        assert_eq!(f["profiles"][0]["name"], "Omarchy Overlay");
        assert!(f.get("defaultProfile").is_none());
    }
    #[test]
    fn terminal_uses_each_themes_own_terminal_colors() {
        let gruvbox = crate::theme_files::built_in("gruvbox").unwrap();
        let f = terminal_fragment("gruvbox", "#000000", "#FFFFFF", "#123456", &gruvbox.colors, Some("Cascadia Mono"));
        assert_eq!(f["profiles"][0]["font"]["face"], "Cascadia Mono");
        let scheme = &f["schemes"][0];
        let resolved = crate::vscode_theme::resolved(&gruvbox.colors);
        assert_eq!(scheme["black"], format!("#{:06X}", gruvbox.colors["background"]));
        assert_eq!(scheme["brightWhite"], format!("#{:06X}", resolved["bright_foreground"]));
        for (key, sources) in TERMINAL_COLORS {
            if let Some(value) = sources.iter().find_map(|k| resolved.get(*k)) {
                assert_eq!(scheme[key], format!("#{value:06X}"), "{key}");
            }
        }
        assert_eq!(scheme["red"], format!("#{:06X}", gruvbox.colors["red"]));
        assert_ne!(scheme["red"], "#F7768E", "not Tokyo Night's red");
        // A key the theme leaves out keeps the default.
        let f = terminal_fragment("x", "#000000", "#FFFFFF", "#123456", &Default::default(), None);
        assert!(f["profiles"][0].get("font").is_none());
        assert_eq!(f["schemes"][0]["red"], "#F7768E");
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Advanced {
    pub active_opacity: u8,
    pub inactive_opacity: u8,
    pub coordinate_windows: bool,
    pub window_backdrop: String,
    pub animations: bool,
    pub animation_duration_ms: u32,
}
impl Advanced {
    pub fn capture(c: &Config) -> Self {
        Self {
            active_opacity: c.active_opacity,
            inactive_opacity: c.inactive_opacity,
            coordinate_windows: c.coordinate_windows,
            window_backdrop: c.window_backdrop.clone(),
            animations: c.animations,
            animation_duration_ms: c.animation_duration_ms,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(50..=100).contains(&self.active_opacity)
            || !(70..=100).contains(&self.inactive_opacity)
            || !(50..=400).contains(&self.animation_duration_ms)
            || !["solid", "mica", "acrylic"].contains(&self.window_backdrop.as_str())
        {
            return Err("Choose valid global opacity, a supported material and animation duration 50–400 ms.".into());
        }
        Ok(())
    }
    pub fn apply(&self, c: &mut Config) {
        c.active_opacity = self.active_opacity;
        c.inactive_opacity = self.inactive_opacity;
        c.coordinate_windows = self.coordinate_windows;
        c.window_backdrop = self.window_backdrop.clone();
        c.animations = self.animations;
        c.animation_duration_ms = self.animation_duration_ms;
    }
}

#[cfg(test)]
mod advanced_tests {
    use super::*;
    #[test]
    fn named_profiles_restore_advanced_choices_and_old_profiles_leave_them_alone() {
        let mut c = Config {
            window_backdrop: "acrylic".into(),
            animations: true,
            active_opacity: 95,
            ..Default::default()
        };
        let p = Profile::capture(&c, "Work".into());
        let p: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        p.validate().unwrap();
        c.window_backdrop = "solid".into();
        c.animations = false;
        p.apply(&mut c).unwrap();
        assert_eq!(c.window_backdrop, "acrylic");
        assert!(c.animations);
        assert_eq!(c.active_opacity, 95);
        let mut legacy = serde_json::to_value(&p).unwrap();
        legacy.as_object_mut().unwrap().remove("advanced");
        let old: Profile = serde_json::from_value(legacy).unwrap();
        c.window_backdrop = "mica".into();
        old.apply(&mut c).unwrap();
        assert_eq!(c.window_backdrop, "mica");
    }
}

#[cfg(test)]
mod theme_list_tests {
    use super::*;
    #[test]
    fn every_offered_theme_is_a_real_theme() {
        let ids = THEMES.iter().map(|(id, _)| *id).chain(COMMAND_THEMES);
        for id in ids {
            assert!(crate::theme_files::resolve(id).is_some(), "{id}");
        }
    }
}
