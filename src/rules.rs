use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Tile,
    Float,
    Ignore,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppRule {
    pub executable: String,
    pub title_contains: String,
    pub window_class: String,
    pub mode: Mode,
    pub allow_opacity: bool,
    pub monitor: Option<String>,
    pub active_opacity: Option<u8>,
    pub inactive_opacity: Option<u8>,
    pub coordinate_theme: Option<bool>,
    pub backdrop: Option<String>,
}
impl Default for AppRule {
    fn default() -> Self {
        Self {
            executable: String::new(),
            title_contains: String::new(),
            window_class: String::new(),
            mode: Mode::Tile,
            allow_opacity: true,
            monitor: None,
            active_opacity: None,
            inactive_opacity: None,
            coordinate_theme: None,
            backdrop: None,
        }
    }
}
impl AppRule {
    pub fn valid(&self) -> bool {
        self.backdrop
            .as_ref()
            .is_none_or(|s| ["native", "mica", "acrylic"].contains(&s.as_str()))
            && self.active_opacity.is_none_or(|p| (50..=100).contains(&p))
            && self
                .inactive_opacity
                .is_none_or(|p| (70..=100).contains(&p))
            && !self.executable.trim().is_empty()
            && self.executable.to_lowercase().ends_with(".exe")
            && !self
                .executable
                .chars()
                .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '\0' | '"'))
            && self
                .monitor
                .as_ref()
                .is_none_or(|m| m.starts_with("\\\\.\\DISPLAY"))
    }
    #[cfg(windows)]
    pub fn label(&self) -> String {
        let mode = match self.mode {
            Mode::Tile => "Tile",
            Mode::Float => "Float",
            Mode::Ignore => "Ignore",
        };
        format!(
            "{} ({}) — {}{}{}",
            app_name(&self.executable),
            self.executable,
            mode,
            if self.title_contains.is_empty() && self.window_class.is_empty() {
                ""
            } else {
                " · filter"
            },
            if self.allow_opacity && self.mode != Mode::Ignore {
                ""
            } else {
                " · native opacity"
            }
        )
    }
}
pub fn matching<'a>(
    rules: &'a [AppRule],
    exe: &str,
    title: &str,
    class: &str,
) -> Option<&'a AppRule> {
    let title = title.to_lowercase();
    rules
        .iter()
        .enumerate()
        .filter(|(_, rule)| {
            rule.executable.eq_ignore_ascii_case(exe)
                && (rule.title_contains.is_empty()
                    || title.contains(&rule.title_contains.to_lowercase()))
                && (rule.window_class.is_empty() || rule.window_class.eq_ignore_ascii_case(class))
        })
        .max_by_key(|(i, rule)| {
            (
                usize::from(!rule.title_contains.is_empty())
                    + usize::from(!rule.window_class.is_empty()),
                std::cmp::Reverse(*i),
            )
        })
        .map(|(_, rule)| rule)
}
#[cfg(windows)]
pub fn work_starters() -> Vec<AppRule> {
    [
        ("msedge.exe", Mode::Tile, true),
        ("explorer.exe", Mode::Tile, true),
        ("EXCEL.EXE", Mode::Float, false),
        ("OUTLOOK.EXE", Mode::Float, false),
        ("olk.exe", Mode::Float, false),
    ]
    .into_iter()
    .map(|(exe, mode, opacity)| AppRule {
        executable: exe.into(),
        mode,
        allow_opacity: opacity,
        ..AppRule::default()
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn specific_dialog_rule_wins_and_executables_are_case_insensitive() {
        let rules = vec![
            AppRule {
                executable: "excel.exe".into(),
                ..AppRule::default()
            },
            AppRule {
                executable: "EXCEL.EXE".into(),
                title_contains: "options".into(),
                mode: Mode::Float,
                ..AppRule::default()
            },
        ];
        assert_eq!(
            matching(&rules, "EXCEL.EXE", "Book 1", "XLMAIN")
                .unwrap()
                .mode,
            Mode::Tile
        );
        assert_eq!(
            matching(&rules, "excel.exe", "Excel OPTIONS", "XLMAIN")
                .unwrap()
                .mode,
            Mode::Float
        );
        assert!(matching(&rules, "explorer.exe", "Excel Options", "CabinetWClass").is_none());
    }
    #[test]
    fn exact_class_filter_and_stable_precedence() {
        let first = AppRule {
            executable: "test.exe".into(),
            window_class: "MainWindow".into(),
            mode: Mode::Ignore,
            ..AppRule::default()
        };
        let second = AppRule {
            mode: Mode::Float,
            ..first.clone()
        };
        assert_eq!(
            matching(&[first.clone(), second], "TEST.exe", "Title", "mainwindow")
                .unwrap()
                .mode,
            Mode::Ignore
        );
        assert!(matching(&[first], "test.exe", "Title", "Dialog").is_none());
    }
    #[test]
    fn rejects_paths_and_accepts_real_executable_names() {
        for name in ["", "test", "C:\\test.exe", "../test.exe", "*.exe"] {
            assert!(!AppRule {
                executable: name.into(),
                ..AppRule::default()
            }
            .valid());
        }
        assert!(AppRule {
            executable: "My Editor.exe".into(),
            ..AppRule::default()
        }
        .valid());
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub mode: Mode,
    pub allow_opacity: bool,
    pub active_opacity: u8,
    pub inactive_opacity: u8,
    pub coordinate_theme: bool,
    pub backdrop: u32,
    pub material_explicit: bool,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Tile",
            Self::Float => "Float",
            Self::Ignore => "Ignore",
        }
    }
}
pub fn app_name(executable: &str) -> &str {
    match executable.to_ascii_lowercase().as_str() {
        "olk.exe" => "Outlook (new)",
        "outlook.exe" => "Outlook (classic)",
        "excel.exe" => "Excel",
        "msedge.exe" => "Microsoft Edge",
        "explorer.exe" => "File Explorer",
        "chrome.exe" => "Google Chrome",
        _ => executable,
    }
}
