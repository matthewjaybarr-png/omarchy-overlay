//! config\menu.json: the Omarchy-style menu tree. Each item runs one command from
//! commands.rs; submenus are `menu <id>`. A broken file is rejected whole with the
//! line that is wrong, and Omarchy Overlay keeps the built-in menu.
use crate::commands::{self, Command};
use serde::Deserialize;

pub const DEFAULT: &str = include_str!("../config/examples/menu.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    menus: Vec<RawMenu>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMenu {
    id: String,
    title: String,
    items: Vec<RawItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawItem {
    label: String,
    #[serde(default)]
    icon: String,
    command: String,
    #[serde(default)]
    confirm: bool,
    /// Shows the small "Windows" tag. Left out, it follows the command.
    #[serde(default)]
    windows: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub label: String,
    pub icon: Option<char>,
    pub command: Command,
    pub confirm: bool,
    pub windows: bool,
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub id: String,
    pub title: String,
    pub items: Vec<Item>,
}
#[derive(Clone, Debug)]
pub struct Menus {
    pub menus: Vec<Menu>,
}
impl Menus {
    pub fn get(&self, id: &str) -> Option<&Menu> {
        self.menus.iter().find(|m| m.id == id)
    }
}

/// The first line holding every piece of `needles`, so a mistake found after
/// reading can still point at its line.
fn line_of(text: &str, needles: &[&str]) -> usize {
    text.lines()
        .position(|line| needles.iter().all(|n| line.contains(n)))
        .map_or(1, |i| i + 1)
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

pub fn icon(hex: &str) -> Result<Option<char>, String> {
    if hex.trim().is_empty() {
        return Ok(None);
    }
    u32::from_str_radix(hex.trim().trim_start_matches("U+"), 16)
        .ok()
        .and_then(char::from_u32)
        .filter(|c| !c.is_control())
        .map(Some)
        .ok_or_else(|| format!("\"{hex}\" is not an icon code. Use a hex code such as F011."))
}

pub fn parse(text: &str) -> Result<Menus, String> {
    let file: File = serde_json::from_str(text)
        .map_err(|e| format!("menu.json line {}: {e}", e.line().max(1)))?;
    if file.menus.is_empty() || file.menus.len() > 64 {
        return Err("menu.json line 1: write 1 to 64 menus.".into());
    }
    let ids: Vec<String> = file
        .menus
        .iter()
        .map(|m| m.id.to_ascii_lowercase())
        .collect();
    let mut menus = vec![];
    for (index, raw) in file.menus.into_iter().enumerate() {
        let at = |needles: &[&str], message: String| {
            format!("menu.json line {}: {message}", line_of(text, needles))
        };
        let id = &ids[index];
        let id_text = quoted(&raw.id);
        if id.is_empty()
            || id.len() > 40
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(at(
                &["\"id\"", &id_text],
                "menu ids use letters, digits and dashes.".into(),
            ));
        }
        if ids[..index].contains(id) {
            return Err(at(
                &["\"id\"", &id_text],
                format!("the menu id \"{id}\" is used twice."),
            ));
        }
        if raw.title.trim().is_empty() || raw.title.chars().count() > 60 {
            return Err(at(
                &["\"id\"", &id_text],
                "the menu title must be 1 to 60 characters.".into(),
            ));
        }
        if raw.items.is_empty() || raw.items.len() > 60 {
            return Err(at(
                &["\"id\"", &id_text],
                "a menu holds 1 to 60 items.".into(),
            ));
        }
        let mut items = vec![];
        for item in raw.items {
            let label_text = quoted(&item.label);
            let at = |message: String| at(&["\"label\"", &label_text], message);
            if item.label.trim().is_empty()
                || item.label.chars().count() > 60
                || item.label.chars().any(char::is_control)
            {
                return Err(at("the label must be 1 to 60 characters.".into()));
            }
            let command = commands::parse(&item.command).map_err(at)?;
            if let Command::Menu(target) = &command {
                if !ids.contains(target) {
                    return Err(at(format!("there is no menu with the id \"{target}\".")));
                }
            }
            items.push(Item {
                icon: icon(&item.icon).map_err(at)?,
                windows: item.windows.unwrap_or_else(|| command.windows()),
                label: item.label,
                command,
                confirm: item.confirm,
            });
        }
        menus.push(Menu {
            id: id.clone(),
            title: raw.title,
            items,
        });
    }
    if !ids.iter().any(|id| id == "main") {
        return Err("menu.json line 1: one menu must have the id \"main\".".into());
    }
    Ok(Menus { menus })
}

pub fn defaults() -> Menus {
    parse(DEFAULT).expect("built-in menu.json is valid")
}

/// A path pasted with one pair of surrounding double quotes (Explorer's "Copy as
/// path"), without them.
pub fn unquote(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .filter(|t| !t.contains('"'))
        .map_or(text, str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_menu_matches_the_laptop_and_every_target_exists() {
        let menus = defaults();
        let main = menus.get("main").unwrap();
        let labels: Vec<_> = main.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Apps",
                "Learn",
                "Appearance",
                "Trigger",
                "Setup",
                "Install",
                "Remove",
                "Update",
                "About",
                "System"
            ]
        );
        let system = menus.get("system").unwrap();
        assert!(system.items.iter().skip(1).all(|i| i.confirm));
        assert_eq!(system.items[0].icon, Some('\u{f023}'));
        // Every `menu x` a keybind opens must exist.
        for bind in crate::binds::defaults() {
            if let Command::Menu(id) = &bind.command {
                assert!(menus.get(id).is_some(), "{id}");
            }
        }
        let monitors = menus
            .get("setup")
            .unwrap()
            .items
            .iter()
            .find(|i| i.label == "Monitors")
            .unwrap();
        assert!(monitors.windows);
        // The Windows pages the 2026-11-03 request lists open from the menu.
        let commands: Vec<String> = menus
            .menus
            .iter()
            .flat_map(|m| m.items.iter())
            .filter_map(|i| match &i.command {
                Command::Settings(page) => Some(page.clone()),
                _ => None,
            })
            .collect();
        for page in [
            "sound",
            "network-wifi",
            "bluetooth",
            "powersleep",
            "display",
            "keyboard",
            "mousetouchpad",
            "defaultapps",
            "startupapps",
            "fonts",
            "appsfeatures",
            "windowsupdate",
            "nightlight",
        ] {
            assert!(commands.iter().any(|c| c == page), "{page}");
        }
    }
    #[test]
    fn broken_menus_name_the_line() {
        let bad = "{\n \"menus\": [\n  {\"id\": \"main\", \"title\": \"Go\", \"items\": [\n   {\"label\": \"Gone\", \"command\": \"menu nowhere\"}\n  ]}\n ]\n}";
        let err = parse(bad).unwrap_err();
        assert!(err.contains("line 4") && err.contains("nowhere"), "{err}");
        let err = parse("{\"menus\": [\n{\"id\": \"main\", \"title\": \"Go\", \"items\": [}]}")
            .unwrap_err();
        assert!(err.starts_with("menu.json line 2"), "{err}");
        let no_main = "{\"menus\": [{\"id\": \"other\", \"title\": \"X\", \"items\": [{\"label\": \"A\", \"command\": \"apps\"}]}]}";
        assert!(parse(no_main).unwrap_err().contains("main"));
        assert!(icon("ZZ").is_err());
        assert_eq!(icon("F0425").unwrap(), Some('\u{f0425}'));
    }
    #[test]
    fn pasted_paths_lose_their_quotes() {
        assert_eq!(unquote(r#""C:\Tools\app.exe""#), r"C:\Tools\app.exe");
        assert_eq!(unquote(r"tools\fixture.exe"), r"tools\fixture.exe");
        assert_eq!(unquote(r#""a" "b""#), r#""a" "b""#);
        assert_eq!(unquote("\""), "\"");
    }
}
