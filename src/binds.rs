//! config\binds.conf: one keybind per line as `key, description, command`.
//! Super is the Windows key. Keys Windows never gives up and Ctrl-only program
//! shortcuts are refused; plain Alt keys work but are logged as a warning.
use crate::commands::{self, Command};

pub const DEFAULT: &str = include_str!("../config/examples/binds.conf");

pub const ALT: u32 = 1;
pub const CTRL: u32 = 2;
pub const SHIFT: u32 = 4;
pub const SUPER: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub modifiers: u32,
    pub vk: u32,
}

#[derive(Clone, Debug)]
pub struct Bind {
    pub key: Key,
    pub description: String,
    pub command: Command,
    pub line: usize,
}

const NAMES: &[(&str, u32)] = &[
    ("Space", 0x20),
    ("Return", 0x0d),
    ("Enter", 0x0d),
    ("Escape", 0x1b),
    ("Esc", 0x1b),
    ("Backspace", 0x08),
    ("Tab", 0x09),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("Comma", 0xbc),
    ("Period", 0xbe),
    ("Slash", 0xbf),
    ("Grave", 0xc0),
    ("Minus", 0xbd),
    ("Equal", 0xbb),
    ("Semicolon", 0xba),
    ("Apostrophe", 0xde),
    ("BracketLeft", 0xdb),
    ("BracketRight", 0xdd),
    ("Backslash", 0xdc),
    ("Print", 0x2c),
    ("Home", 0x24),
    ("End", 0x23),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("Insert", 0x2d),
    ("Delete", 0x2e),
];

fn key_code(name: &str) -> Option<u32> {
    if let Some((_, vk)) = NAMES.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        return Some(*vk);
    }
    let upper = name.to_ascii_uppercase();
    if upper.len() == 1 && upper.as_bytes()[0].is_ascii_alphanumeric() {
        return Some(upper.as_bytes()[0] as u32);
    }
    upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| (1..=24).contains(n))
        .map(|n| 0x70 + n - 1)
}

fn key_name(vk: u32) -> String {
    if let Some((name, _)) = NAMES.iter().find(|(_, code)| *code == vk) {
        return (*name).into();
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5a => char::from(vk as u8).to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x6f),
        _ => format!("Key{vk:02X}"),
    }
}

impl Key {
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<_> = text.split('+').map(str::trim).collect();
        if parts.iter().any(|p| p.is_empty()) {
            return Err(format!(
                "\"{text}\" is not a key. Write keys like Super+Shift+A."
            ));
        }
        let mut modifiers = 0;
        for part in &parts[..parts.len() - 1] {
            let bit = match part.to_ascii_lowercase().as_str() {
                "super" | "win" => SUPER,
                "ctrl" | "control" => CTRL,
                "alt" => ALT,
                "shift" => SHIFT,
                other => return Err(format!("\"{other}\" is not Super, Ctrl, Alt or Shift.")),
            };
            if modifiers & bit != 0 {
                return Err(format!("\"{part}\" appears twice in {text}."));
            }
            modifiers |= bit;
        }
        let last = parts[parts.len() - 1];
        let vk = key_code(last).ok_or_else(|| format!("\"{last}\" is not a key name."))?;
        let key = Self { modifiers, vk };
        if modifiers & (SUPER | ALT) == 0 {
            return Err("Use Super, Alt or Ctrl+Alt, so program shortcuts stay free.".into());
        }
        if modifiers == SUPER && vk == 0x4c {
            return Err("Super+L always locks Windows and cannot be used.".into());
        }
        if modifiers & (CTRL | ALT) == CTRL | ALT && vk == 0x2e {
            return Err("Ctrl+Alt+Delete always belongs to Windows.".into());
        }
        Ok(key)
    }
    /// A plain-Alt key works but takes the shortcut from the focused program.
    pub fn warning(self) -> Option<&'static str> {
        (self.modifiers & (SUPER | CTRL) == 0)
            .then_some("Plain Alt keys take the shortcut from programs that use them.")
    }
    pub fn label(self) -> String {
        let mut parts = vec![];
        for (bit, name) in [
            (SUPER, "Super"),
            (CTRL, "Ctrl"),
            (ALT, "Alt"),
            (SHIFT, "Shift"),
        ] {
            if self.modifiers & bit != 0 {
                parts.push(name.to_string());
            }
        }
        parts.push(key_name(self.vk));
        parts.join("+")
    }
}

/// Reads binds.conf. Any mistake rejects the whole file with its line number,
/// so Omarchy Overlay keeps the built-in binds instead of running half a keymap.
pub fn parse(text: &str) -> Result<Vec<Bind>, String> {
    let mut binds: Vec<Bind> = vec![];
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let at = |message: String| format!("binds.conf line {line}: {message}");
        let mut fields = content.splitn(3, ',').map(str::trim);
        let (Some(key), Some(description), Some(command)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return Err(at(
                "write the key, a description and the command, separated by commas.".into(),
            ));
        };
        if description.is_empty() || description.len() > 80 {
            return Err(at("the description must be 1 to 80 characters.".into()));
        }
        let key = Key::parse(key).map_err(at)?;
        let command = commands::parse(command).map_err(at)?;
        if let Some(first) = binds.iter().find(|b| b.key == key) {
            return Err(at(format!(
                "{} is already used on line {}.",
                key.label(),
                first.line
            )));
        }
        if binds.len() >= 256 {
            return Err(at("up to 256 keybinds are supported.".into()));
        }
        binds.push(Bind {
            key,
            description: description.into(),
            command,
            line,
        });
    }
    Ok(binds)
}

pub fn defaults() -> Vec<Bind> {
    parse(DEFAULT).expect("built-in binds.conf is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_binds_follow_the_omarchy_laptop() {
        let binds = defaults();
        let find = |key: &str| {
            let key = Key::parse(key).unwrap();
            binds
                .iter()
                .find(|b| b.key == key)
                .map(|b| b.command.clone())
        };
        assert_eq!(find("Super+Space"), Some(Command::Builtin("menu")));
        assert_eq!(find("Super+Alt+Space"), Some(Command::Builtin("apps")));
        assert_eq!(find("Super+K"), Some(Command::Builtin("keybinds")));
        // Omarchy's universal clipboard and window keys.
        assert_eq!(find("Super+C"), Some(Command::Builtin("copy")));
        assert_eq!(find("Super+V"), Some(Command::Builtin("paste")));
        assert_eq!(find("Super+X"), Some(Command::Builtin("cut")));
        assert_eq!(find("Super+A"), Some(Command::Builtin("select-all")));
        assert_eq!(find("Super+M"), Some(Command::Builtin("minimize-window")));
        assert_eq!(find("Super+Ctrl+Tab"), Some(Command::Builtin("former-desktop")));
        assert_eq!(find("Super+Shift+Space"), Some(Command::Builtin("toggle-bar")));
        assert_eq!(find("Super+Ctrl+Space"), Some(Command::Page("background")));
        assert!(matches!(find("Super+W"), Some(Command::Action(_))));
        assert_eq!(find("Super+W"), find("Super+Q"));
        assert_eq!(
            find("Super+Shift+A"),
            Some(Command::WebApp("ChatGPT".into()))
        );
        assert_eq!(find("Super+Shift+S"), Some(Command::WebApp("Google Maps".into())));
        assert_eq!(find("Super+3"), Some(Command::Workspace(3)));
        assert_eq!(find("Super+Ctrl+O"), Some(Command::Menu("toggle".into())));
        assert_eq!(find("Super+Ctrl+Alt+D"), Some(Command::App("calendar")));
        // Personal keys that are not Omarchy's are gone; Super+E is Windows' Explorer again.
        for key in ["Super+E", "Super+Z", "Super+B", "Super+Alt+W", "Super+Shift+V"] {
            assert_eq!(find(key), None, "{key}");
        }
        assert!(matches!(find("Super+Backspace"), Some(Command::Action(_))));
        // Windows keeps Super+Ctrl+D (New desktop): workspaces are Windows desktops.
        assert_eq!(find("Super+Ctrl+D"), None);
        // Opacity toggles are off by default.
        assert_eq!(find("Super+Alt+T"), None);
        let apps = crate::webapps::defaults();
        for bind in &binds {
            assert!(bind.key.modifiers & SUPER != 0, "{}", bind.key.label());
            if let Command::WebApp(name) = &bind.command {
                assert!(apps.iter().any(|a| a.name == *name), "webapps.json lacks {name}");
            }
        }
    }
    #[test]
    fn plain_alt_is_allowed_with_a_warning() {
        let key = Key::parse("Alt+J").unwrap();
        assert!(key.warning().unwrap().contains("Alt"));
        assert_eq!(Key::parse("Super+J").unwrap().warning(), None);
        assert!(Key::parse("Ctrl+J").is_err());
        assert!(Key::parse("Super+L").is_err());
        assert!(Key::parse("Ctrl+Alt+Delete").is_err());
    }
    #[test]
    fn broken_lines_name_the_line_and_reserved_keys_are_refused() {
        assert_eq!(
            Key::parse("super+shift+return").unwrap().label(),
            "Super+Shift+Return"
        );
        let err =
            parse("# note\n\nSuper+B, Browser, app browser\nSuper+B, Again, apps").unwrap_err();
        assert!(err.contains("line 4") && err.contains("line 3"), "{err}");
        assert!(parse("Super+L, Lock, lock").unwrap_err().contains("line 1"));
        assert_eq!(parse("Alt+E, Hide, apps").unwrap().len(), 1);
        assert!(parse("Ctrl+C, Copy, apps").is_err());
        assert!(parse("Ctrl+Alt+Delete, No, apps").is_err());
        assert!(parse("Super+Q, Quit").unwrap_err().contains("commas"));
        assert!(parse("Super+Q, Quit, format c:")
            .unwrap_err()
            .contains("Unknown command"));
        assert!(parse("Super+Hyper, X, apps").is_err());
        assert_eq!(
            parse("Ctrl+Alt+T, Tiling, action toggle-tiling")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            parse("Super+Q, Run, launch \"tools\\a b.cmd\", x").unwrap()[0].command,
            Command::Launch {
                program: "tools\\a b.cmd".into(),
                arguments: ", x".into()
            }
        );
    }
}
