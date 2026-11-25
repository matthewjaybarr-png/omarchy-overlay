//! Color themes in the Omarchy colors.toml format: embedded ones from
//! themes/omarchy, plus config\themes\<id>.toml or config\themes\<id>\colors.toml.
//! Known color keys must be `key = "#rrggbb"`; a mistake names the file and line.
//! Unknown keys are ignored so newer Omarchy files still load.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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
    /// Every color key in the file, as 0xrrggbb.
    pub colors: BTreeMap<String, u32>,
    /// The theme's backgrounds folder, for user theme folders that have one.
    pub backgrounds: Option<PathBuf>,
}

/// The colors.toml keys Omarchy defines; any other key is ignored.
const KNOWN: &[&str] = &[
    "accent", "selection", "muted", "background", "dark_background", "darker_background",
    "lighter_background", "foreground", "dark_foreground", "light_foreground",
    "bright_foreground", "cursor", "selection_foreground", "selection_background",
    "red", "yellow", "orange", "green", "cyan", "blue", "magenta", "brown", "black", "white",
    "bright_red", "bright_yellow", "bright_orange", "bright_green", "bright_cyan",
    "bright_blue", "bright_magenta", "bright_brown", "bright_black", "bright_white",
];

#[cfg(test)]
pub static TEST_LOCK: Mutex<()> = Mutex::new(());

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
    let mut values = BTreeMap::new();
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
        let key = key.trim();
        if key != "mode" && !KNOWN.contains(&key) {
            continue;
        }
        let value = rest
            .trim()
            .strip_prefix('"')
            .and_then(|v| v.split_once('"'))
            .map(|(v, _)| v)
            .ok_or_else(|| at("write values in quotes, like accent = \"#38bdfa\".".into()))?;
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
        colors: values,
        backgrounds: None,
    })
}

/// The embedded theme with this id.
pub fn built_in(id: &str) -> Option<Theme> {
    static PARSED: OnceLock<Vec<Theme>> = OnceLock::new();
    PARSED
        .get_or_init(|| {
            crate::omarchy_themes::FILES
                .iter()
                .filter_map(|(id, text)| parse(id, text).ok())
                .collect()
        })
        .iter()
        .find(|t| t.id == id)
        .cloned()
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
/// A user theme first, then an embedded one.
pub fn resolve(id: &str) -> Option<Theme> {
    find(id).or_else(|| built_in(id))
}
pub fn loaded() -> Vec<Theme> {
    LOADED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
/// A theme's preview swatch as COLORREFs: background, accent, foreground, and its
/// red or a second color, so themes with the same background still look different.
pub fn preview(id: &str) -> Option<[u32; 4]> {
    let t = resolve(id)?;
    let colorref = |v: u32| ((v & 255) << 16) | (v & 0xff00) | ((v >> 16) & 255);
    let fourth = ["red", "green", "magenta", "yellow"]
        .iter()
        .find_map(|k| t.colors.get(*k).copied())
        .unwrap_or(t.surface);
    Some([t.background, t.accent, t.text, fourth].map(colorref))
}
/// The image after `current` in a theme's backgrounds, wrapping; the first when
/// `current` is not one of them.
pub fn next_background(list: &[PathBuf], current: Option<&Path>) -> Option<PathBuf> {
    let at = list.iter().position(|p| Some(p.as_path()) == current);
    list.get(at.map_or(0, |i| (i + 1) % list.len())).cloned()
}
/// Registry values for the Windows accent from a COLORREF (0x00BBGGRR), in the order of
/// windows_accent's VALUES: ABGR for AccentColor and the two menu values, ARGB with
/// Windows' default 0xC4 alpha for ColorizationColor and ColorizationAfterglow.
pub fn accent_values(accent: u32) -> [u32; 5] {
    let abgr = 0xFF00_0000 | (accent & 0xFF_FFFF);
    let argb = 0xC400_0000 | ((accent & 255) << 16) | (accent & 0xff00) | ((accent >> 16) & 255);
    [abgr, argb, argb, abgr, abgr]
}
/// The binary AccentPalette for a COLORREF accent: eight RGBA colors, three lighter
/// shades, the accent, three darker shades, then `last` (Windows keeps a contrast
/// color there; the caller passes the one already set). Start and the taskbar use the
/// darker shades in dark mode and the lighter ones in light mode.
pub fn accent_palette(accent: u32, last: [u8; 4]) -> [u8; 32] {
    let rgb = [accent & 255, (accent >> 8) & 255, (accent >> 16) & 255];
    let toward = |target: u32, amount: f64| {
        rgb.map(|c| (c as f64 + (target as f64 - c as f64) * amount + 0.5) as u8)
    };
    let shades = [
        toward(255, 0.70),
        toward(255, 0.45),
        toward(255, 0.25),
        rgb.map(|c| c as u8),
        toward(0, 0.20),
        toward(0, 0.40),
        toward(0, 0.60),
    ];
    let mut out = [0u8; 32];
    for (i, [r, g, b]) in shades.into_iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&[r, g, b, 0]);
    }
    out[28..].copy_from_slice(&last);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accent_palette_brackets_the_accent_with_lighter_and_darker_shades() {
        // Tokyo Night accent #7aa2f7 as a COLORREF.
        let p = accent_palette(0x00f7a27a, [0xf7, 0x63, 0x0c, 0]);
        assert_eq!(&p[12..16], &[0x7a, 0xa2, 0xf7, 0]);
        let luma = |i: usize| p[i * 4] as u32 + p[i * 4 + 1] as u32 + p[i * 4 + 2] as u32;
        for i in 0..6 {
            assert!(luma(i) > luma(i + 1), "shade {i}");
        }
        assert_eq!(&p[28..], &[0xf7, 0x63, 0x0c, 0]);
        assert!(p.iter().skip(3).step_by(4).take(7).all(|&a| a == 0));
    }
    #[test]
    fn preview_uses_background_accent_foreground_and_red() {
        let p = preview("tokyo-night").unwrap();
        // #1a1b26, #7aa2f7, #a9b1d6, #f7768e as COLORREFs.
        assert_eq!(p, [0x00261b1a, 0x00f7a27a, 0x00d6b1a9, 0x008e76f7]);
        assert!(preview("no-such-theme").is_none());
    }
    #[test]
    fn next_background_starts_at_the_first_and_wraps() {
        let list: Vec<PathBuf> = ["a.jpg", "b.jpg", "c.jpg"].map(PathBuf::from).into();
        assert_eq!(next_background(&[], None), None);
        assert_eq!(next_background(&list, None), Some("a.jpg".into()));
        assert_eq!(next_background(&list, Some(Path::new("x.jpg"))), Some("a.jpg".into()));
        assert_eq!(next_background(&list, Some(Path::new("a.jpg"))), Some("b.jpg".into()));
        assert_eq!(next_background(&list, Some(Path::new("c.jpg"))), Some("a.jpg".into()));
    }
    #[test]
    fn accent_values_use_abgr_and_argb() {
        // Tokyo Night accent #7aa2f7 as a COLORREF.
        let v = accent_values(0x00f7a27a);
        assert_eq!(v, [0xFFf7a27a, 0xC47aa2f7, 0xC47aa2f7, 0xFFf7a27a, 0xFFf7a27a]);
    }
    #[test]
    fn every_built_in_theme_parses() {
        for (id, text) in crate::omarchy_themes::FILES {
            parse(id, text).unwrap_or_else(|e| panic!("{e}"));
        }
        let neon = built_in("neon-ridge").unwrap();
        assert_eq!(neon.accent, 0x38bdfa);
        assert_eq!(neon.surface, 0x1a2550);
        assert_eq!(neon.colors["magenta"], 0xb580fa);
    }
    #[test]
    fn unknown_keys_are_ignored() {
        let t = parse(
            "x",
            "background=\"#000000\"\nforeground=\"#ffffff\"\naccent=\"#ff0000\"\nnew_thing = 3\n",
        )
        .unwrap();
        assert_eq!(t.accent, 0xff0000);
        let err = parse("x", "background = \"#000000\"\nred = 3\n").unwrap_err();
        assert!(err.contains("line 2"), "{err}");
    }
    #[test]
    fn user_theme_overrides_built_in() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mine = parse(
            "tokyo-night",
            "background = \"#000000\"\nforeground = \"#ffffff\"\naccent = \"#123456\"",
        )
        .unwrap();
        install(vec![mine]);
        assert_eq!(resolve("tokyo-night").unwrap().accent, 0x123456);
        install(vec![]);
        assert_ne!(resolve("tokyo-night").unwrap().accent, 0x123456);
    }
    #[test]
    fn removed_ids_fall_back() {
        // theme::set falls back to neon-ridge when resolve finds nothing.
        assert!(resolve("paper").is_none());
        assert!(resolve("jade").is_none());
        assert!(resolve("neon-ridge").is_some());
        assert_eq!(crate::omarchy_themes::NAMES[0].0, "neon-ridge");
    }
    #[test]
    fn colors_toml_files_load_and_mistakes_name_the_line() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let neon = crate::omarchy_themes::FILES.iter().find(|(id, _)| *id == "neon-ridge").unwrap().1;
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
