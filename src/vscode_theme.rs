//! VS Code (and Insiders, VSCodium, Cursor) themed like Omarchy's
//! omarchy-theme-set-vscode: the theme's colors fill Omarchy's color theme template,
//! which is installed as a local extension, and settings.json's
//! `workbench.colorTheme` names it. settings.json is JSONC, so only that one value is
//! edited in place; comments and the rest of the file stay as they are.
use std::collections::BTreeMap;

/// The theme label VS Code lists, and the extension that provides it.
pub const THEME_LABEL: &str = "Omarchy Overlay";
pub const EXTENSION_NAME: &str = "omarchy-overlay-theme";
pub const EXTENSION_ID: &str = "local.omarchy-overlay-theme";
const TEMPLATE: &str = include_str!("../assets/vscode/omarchy-color-theme.json.tpl");

fn hex(value: u32) -> String {
    format!("#{value:06x}")
}
/// A blend of two colors, `amount` (0–1) of the way from `start` to `end`.
pub fn mix(start: u32, end: u32, amount: f64) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let a = ((start >> shift) & 255) as f64;
        let b = ((end >> shift) & 255) as f64;
        ((a * (1.0 - amount) + b * amount + 0.5) as u32).min(255)
    };
    channel(16) << 16 | channel(8) << 8 | channel(0)
}
/// Every color Omarchy's templates use, with Omarchy's own fallbacks
/// (bin/omarchy-theme-color) for keys a colors.toml leaves out.
pub fn resolved(colors: &BTreeMap<String, u32>) -> BTreeMap<String, u32> {
    let mut c = colors.clone();
    let alias = |c: &mut BTreeMap<String, u32>, key: &str, from: &str| {
        if !c.contains_key(key) {
            if let Some(&v) = c.get(from) {
                c.insert(key.into(), v);
            }
        }
    };
    alias(&mut c, "magenta", "purple");
    alias(&mut c, "bright_magenta", "bright_purple");
    alias(&mut c, "light_foreground", "foreground");
    alias(&mut c, "bright_foreground", "foreground");
    if let Some(&v) = c.get("bright_foreground") {
        c.insert("cursor".into(), v);
    }
    alias(&mut c, "lighter_background", "background");
    alias(&mut c, "dark_foreground", "foreground");
    alias(&mut c, "muted", "dark_foreground");
    alias(&mut c, "selection", "selection_background");
    alias(&mut c, "selection", "background");
    alias(&mut c, "selection_background", "selection");
    alias(&mut c, "selection_foreground", "bright_foreground");
    alias(&mut c, "orange", "yellow");
    let derive = |c: &mut BTreeMap<String, u32>, key: &str, from: &str, toward: u32, amount: f64| {
        if !c.contains_key(key) {
            if let Some(&v) = c.get(from) {
                c.insert(key.into(), mix(v, toward, amount));
            }
        }
    };
    derive(&mut c, "brown", "orange", 0x000000, 0.5);
    derive(&mut c, "dark_background", "background", 0x000000, 0.25);
    derive(&mut c, "darker_background", "background", 0x000000, 0.5);
    for color in ["red", "yellow", "green", "cyan", "blue", "magenta"] {
        derive(&mut c, &format!("bright_{color}"), color, 0xffffff, 0.2);
    }
    alias(&mut c, "purple", "magenta");
    alias(&mut c, "bright_purple", "bright_magenta");
    alias(&mut c, "accent", "blue");
    c
}
/// Omarchy's VS Code color theme filled from a theme. Unknown placeholders fall back
/// to the foreground color, so a theme missing a key still gives valid JSON.
pub fn render(colors: &BTreeMap<String, u32>, dark: bool) -> String {
    fill(TEMPLATE, colors, dark)
}
/// Omarchy's Obsidian theme (default/themed/obsidian.css.tpl) filled from a theme.
pub fn obsidian_css(colors: &BTreeMap<String, u32>, dark: bool) -> String {
    fill(include_str!("../assets/obsidian/omarchy.css.tpl"), colors, dark)
}
/// Fills an Omarchy template's `{{ key }}` and `{{ mix a b N% }}` placeholders.
pub fn fill(template: &str, colors: &BTreeMap<String, u32>, dark: bool) -> String {
    let colors = resolved(colors);
    let fallback = colors.get("foreground").copied().unwrap_or(0x808080);
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        let token = rest[start + 2..start + end].trim();
        let words: Vec<&str> = token.split_whitespace().collect();
        let value = match words.as_slice() {
            ["theme_type"] => if dark { "dark" } else { "light" }.to_string(),
            ["mix", a, b, amount] => {
                let get = |k: &str| colors.get(k).copied().unwrap_or(fallback);
                let amount = amount.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) / 100.0;
                hex(mix(get(a), get(b), amount))
            }
            [key] => hex(colors.get(*key).copied().unwrap_or(fallback)),
            _ => hex(fallback),
        };
        out.push_str(&value);
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}
/// The extension's package.json. The version follows the colors, because VS Code
/// only reloads a theme when the extension that provides it changes.
pub fn package(theme_json: &str, dark: bool) -> (String, serde_json::Value) {
    let sum = theme_json
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
    let version = format!("1.0.{}", sum % 1_000_000_000);
    let package = serde_json::json!({
        "name": EXTENSION_NAME,
        "displayName": THEME_LABEL,
        "description": "Omarchy Overlay's current theme",
        "publisher": "local",
        "version": version,
        "engines": { "vscode": "^1.70.0" },
        "categories": ["Themes"],
        "contributes": { "themes": [{
            "label": THEME_LABEL,
            "uiTheme": if dark { "vs-dark" } else { "vs" },
            "path": "./themes/omarchy-overlay-color-theme.json"
        }]}
    });
    (version, package)
}
/// The editor's extensions.json with this extension registered (or removed when
/// `entry` is None). Any other entry is kept as it is.
pub fn register(extensions: &str, entry: Option<(&str, &str)>) -> Result<String, String> {
    let mut list: Vec<serde_json::Value> = if extensions.trim().is_empty() {
        vec![]
    } else {
        serde_json::from_str(extensions).map_err(|e| format!("extensions.json: {e}"))?
    };
    list.retain(|e| e["identifier"]["id"].as_str() != Some(EXTENSION_ID));
    if let Some((version, folder)) = entry {
        let path = folder.replace('\\', "/");
        let uri_path = if path.starts_with('/') { path.clone() } else { format!("/{path}") };
        list.push(serde_json::json!({
            "identifier": { "id": EXTENSION_ID },
            "version": version,
            "location": { "$mid": 1, "fsPath": folder, "path": uri_path, "scheme": "file" },
            "relativeLocation": EXTENSION_NAME
        }));
    }
    serde_json::to_string(&list).map_err(|e| e.to_string())
}
/// Byte range of the string value of `"workbench.colorTheme"`, quotes excluded.
fn theme_value(text: &str) -> Option<(usize, usize)> {
    let key = "\"workbench.colorTheme\"";
    let mut from = 0;
    while let Some(at) = text[from..].find(key) {
        let after = from + at + key.len();
        let rest = &text[after..];
        let colon = rest.len() - rest.trim_start().len();
        if rest[colon..].starts_with(':') {
            let value = &rest[colon + 1..];
            let space = value.len() - value.trim_start().len();
            if value[space..].starts_with('"') {
                let start = after + colon + 1 + space + 1;
                let mut escaped = false;
                for (i, ch) in text[start..].char_indices() {
                    match ch {
                        '\\' if !escaped => escaped = true,
                        '"' if !escaped => return Some((start, start + i)),
                        _ => escaped = false,
                    }
                }
                return None;
            }
        }
        from = after;
    }
    None
}
/// The color theme settings.json names now, if it names one.
pub fn color_theme(text: &str) -> Option<String> {
    theme_value(text).map(|(a, b)| text[a..b].to_string())
}
/// settings.json with `workbench.colorTheme` set to `name`, or removed when None.
/// Only that value changes; comments, order and formatting stay.
pub fn set_color_theme(text: &str, name: Option<&str>) -> String {
    let escaped = name.map(|n| n.replace('\\', "\\\\").replace('"', "\\\""));
    match (theme_value(text), escaped) {
        (Some((a, b)), Some(name)) => format!("{}{}{}", &text[..a], name, &text[b..]),
        (Some((a, b)), None) => {
            // Remove `"workbench.colorTheme": "…"` and one comma next to it.
            let key_start = text[..a].rfind("\"workbench.colorTheme\"").unwrap_or(a);
            let mut end = b + 1;
            let tail = &text[end..];
            let skip = tail.len() - tail.trim_start().len();
            if tail[skip..].starts_with(',') {
                end += skip + 1;
                return format!("{}{}", &text[..key_start], text[end..].trim_start_matches([' ', '\t']));
            }
            let head = text[..key_start].trim_end();
            let head = head.strip_suffix(',').unwrap_or(head);
            format!("{}{}", head, &text[end..])
        }
        (None, Some(name)) => match text.find('{') {
            Some(open) => {
                let rest = &text[open + 1..];
                let empty = rest.trim_start().starts_with('}');
                format!(
                    "{}\n    \"workbench.colorTheme\": \"{}\"{}{}",
                    &text[..=open],
                    name,
                    if empty { "\n" } else { "," },
                    if empty { rest.trim_start() } else { rest }
                )
            }
            None => format!("{{\n    \"workbench.colorTheme\": \"{name}\"\n}}\n"),
        },
        (None, None) => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn strict(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }
    #[test]
    fn every_built_in_theme_renders_valid_json_without_placeholders() {
        for (id, _) in crate::omarchy_themes::FILES {
            let theme = crate::theme_files::built_in(id).unwrap();
            let json = render(&theme.colors, theme.dark);
            assert!(!json.contains("{{"), "{id}");
            let value = strict(&json);
            assert_eq!(value["type"], if theme.dark { "dark" } else { "light" }, "{id}");
            assert_eq!(value["colors"]["foreground"], format!("#{:06x}", theme.colors["foreground"]));
        }
    }
    #[test]
    fn obsidian_css_has_no_placeholders_left() {
        let theme = crate::theme_files::built_in("gruvbox").unwrap();
        let css = obsidian_css(&theme.colors, theme.dark);
        assert!(!css.contains("{{"));
        assert!(css.contains(&format!("#{:06x}", theme.colors["red"])));
    }
    #[test]
    fn missing_colors_follow_omarchys_fallbacks() {
        let mut colors = BTreeMap::new();
        colors.insert("background".to_string(), 0x202020);
        colors.insert("foreground".to_string(), 0xe0e0e0);
        colors.insert("red".to_string(), 0xff0000);
        colors.insert("yellow".to_string(), 0xffff00);
        let r = resolved(&colors);
        assert_eq!(r["bright_red"], mix(0xff0000, 0xffffff, 0.2));
        assert_eq!(r["orange"], 0xffff00);
        assert_eq!(r["darker_background"], mix(0x202020, 0, 0.5));
        assert_eq!(r["muted"], 0xe0e0e0);
        assert_eq!(mix(0x000000, 0xffffff, 0.5), 0x808080);
    }
    #[test]
    fn settings_keep_comments_and_other_values() {
        let text = "{\n    // my font\n    \"editor.fontSize\": 14,\n    \"workbench.colorTheme\": \"Default Dark+\",\n    \"files.autoSave\": \"off\",\n}\n";
        assert_eq!(color_theme(text).as_deref(), Some("Default Dark+"));
        let set = set_color_theme(text, Some(THEME_LABEL));
        assert_eq!(set, text.replace("Default Dark+", THEME_LABEL));
        assert_eq!(set_color_theme(&set, Some("Default Dark+")), text);
        let removed = set_color_theme(text, None);
        assert_eq!(color_theme(&removed), None);
        assert!(removed.contains("// my font") && removed.contains("\"files.autoSave\": \"off\""));
    }
    #[test]
    fn settings_without_the_key_get_it_and_lose_it_again() {
        for text in ["{\n    \"editor.fontSize\": 14\n}\n", "{}", "{\n}\n", ""] {
            let set = set_color_theme(text, Some(THEME_LABEL));
            assert_eq!(color_theme(&set).as_deref(), Some(THEME_LABEL), "{text:?}");
            let back = set_color_theme(&set, None);
            assert_eq!(color_theme(&back), None);
            if !text.is_empty() {
                let v = strict(&back);
                assert_eq!(v.get("editor.fontSize").is_some(), text.contains("fontSize"));
            }
        }
        // The last entry: its comma comes before it.
        let text = "{\n  \"a\": 1,\n  \"workbench.colorTheme\": \"X\"\n}";
        assert_eq!(strict(&set_color_theme(text, None)), serde_json::json!({"a": 1}));
        // A quote in a theme name is escaped.
        let odd = set_color_theme("{}", Some("A \"B\""));
        assert_eq!(strict(&odd)["workbench.colorTheme"], "A \"B\"");
        assert_eq!(color_theme(&odd).as_deref(), Some("A \\\"B\\\""));
    }
    #[test]
    fn registration_replaces_only_our_entry() {
        let other = r#"[{"identifier":{"id":"ms-python.python"},"version":"1.0.0"}]"#;
        let added = register(other, Some(("1.0.5", "C:\\Users\\me\\.vscode\\extensions\\omarchy-overlay-theme"))).unwrap();
        let list = strict(&added);
        assert_eq!(list.as_array().unwrap().len(), 2);
        assert_eq!(list[1]["location"]["path"], "/C:/Users/me/.vscode/extensions/omarchy-overlay-theme");
        let again = register(&added, Some(("1.0.6", "C:\\x"))).unwrap();
        assert_eq!(strict(&again).as_array().unwrap().len(), 2);
        assert_eq!(strict(&register(&again, None).unwrap()), strict(other));
        assert!(register("", None).unwrap() == "[]");
        let (v1, _) = package("a", true);
        let (v2, p) = package("b", false);
        assert_ne!(v1, v2);
        assert_eq!(p["contributes"]["themes"][0]["uiTheme"], "vs");
    }
}
