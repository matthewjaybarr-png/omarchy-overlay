//! config\themes\*.toml: extra color themes in the Omarchy colors.toml format.
//! Only `key = "#rrggbb"` lines and comments are read; every other line is an
//! error that names the file and line.
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    pub id: String,
    pub label: String,
    pub background: u32,
    pub surface: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub border: u32,
    pub dark: bool,
}

static LOADED: Mutex<Vec<Theme>> = Mutex::new(vec![]);

/// Theme ids are file names: letters, digits and dashes.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn color(value: &str) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    (hex.len() == 6).then(|| u32::from_str_radix(hex, 16).ok())?
}

pub fn parse(id: &str, text: &str) -> Result<Theme, String> {
    let file = format!("themes\\{id}.toml");
    if !valid_name(id) {
        return Err(format!(
            "{file}: name theme files with lowercase letters, digits and dashes."
        ));
    }
    let mut values = std::collections::HashMap::new();
    let mut mode = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        // Colors start with '#', so only a line that starts with one is a comment.
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let at = |m: String| format!("{file} line {}: {m}", index + 1);
        let (key, rest) = line
            .split_once('=')
            .ok_or_else(|| at("expected key = \"#rrggbb\".".into()))?;
        let value = rest
            .trim()
            .strip_prefix('"')
            .and_then(|v| v.split_once('"'))
            .map(|(v, _)| v)
            .ok_or_else(|| at("write values in quotes, like accent = \"#38bdfa\".".into()))?;
        let key = key.trim();
        if key == "mode" {
            if value != "dark" && value != "light" {
                return Err(at("mode is \"dark\" or \"light\".".into()));
            }
            mode = Some(value == "dark");
        } else {
            let parsed = color(value)
                .ok_or_else(|| at(format!("\"{value}\" is not a color like #38bdfa.")))?;
            values.insert(key.to_string(), parsed);
        }
    }
    let get = |keys: &[&str]| keys.iter().find_map(|k| values.get(*k).copied());
    let need = |key: &str| {
        values
            .get(key)
            .copied()
            .ok_or_else(|| format!("{file}: add a line for {key}."))
    };
    let background = need("background")?;
    let text = need("foreground")?;
    let accent = need("accent")?;
    // Light themes have a light background.
    let brightness = ((background >> 16) & 255) + ((background >> 8) & 255) + (background & 255);
    Ok(Theme {
        label: id
            .split('-')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
        id: id.into(),
        background,
        surface: get(&["lighter_background", "selection"]).unwrap_or(background),
        text,
        muted: get(&["dark_foreground", "muted"]).unwrap_or(text),
        accent,
        border: get(&["selection", "lighter_background"]).unwrap_or(accent),
        dark: mode.unwrap_or(brightness < 384),
    })
}

/// Replaces the loaded theme files.
pub fn install(themes: Vec<Theme>) {
    *LOADED.lock().unwrap_or_else(|e| e.into_inner()) = themes;
}
pub fn find(id: &str) -> Option<Theme> {
    LOADED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|t| t.id == id)
        .cloned()
}
pub fn loaded() -> Vec<Theme> {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_toml_files_load_and_mistakes_name_the_line() {
        let neon = include_str!("../assets/neon-ridge-colors.toml");
        let t = parse("my-neon", neon).unwrap();
        assert_eq!(t.label, "My Neon");
        assert_eq!(t.accent, 0x38bdfa);
        assert_eq!(t.surface, 0x1a2550);
        assert!(t.dark);
        let err = parse("x", "background = \"#000000\"\naccent = blue\n").unwrap_err();
        assert!(err.contains("line 2"), "{err}");
        assert!(parse("x", "background = \"#000000\"")
            .unwrap_err()
            .contains("foreground"));
        assert!(parse("Bad Name", "").is_err());
        let light = parse(
            "p",
            "background = \"#ffffff\"\nforeground = \"#000000\"\naccent = \"#ff0000\"",
        )
        .unwrap();
        assert!(!light.dark);
        install(vec![light]);
        assert!(find("p").is_some());
        install(vec![]);
    }
}
