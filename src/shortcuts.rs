//! Portable shortcut parsing; modifier bits match RegisterHotKey's documented values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub modifiers: u32,
    pub key: u32,
}
pub const DEFAULT_CYCLE: &str = "Ctrl+Alt+F11";

#[derive(Clone, Copy)]
pub struct Action {
    pub id: &'static str,
    pub label: &'static str,
    pub key: &'static str,
    pub shift: bool,
}
pub const ACTIONS: &[Action] = &[
    Action {
        id: "toggle-tiling",
        label: "Toggle tiling",
        key: "T",
        shift: false,
    },
    Action {
        id: "launcher",
        label: "Launcher",
        key: "Space",
        shift: false,
    },
    Action {
        id: "terminal",
        label: "Terminal",
        key: "Enter",
        shift: false,
    },
    Action {
        id: "float",
        label: "Float",
        key: "F",
        shift: false,
    },
    Action {
        id: "help",
        label: "Help",
        key: "/",
        shift: false,
    },
    Action {
        id: "exit",
        label: "Exit",
        key: "Escape",
        shift: false,
    },
    Action {
        id: "focus-left",
        label: "Focus left",
        key: "H",
        shift: false,
    },
    Action {
        id: "focus-down",
        label: "Focus down",
        key: "J",
        shift: false,
    },
    Action {
        id: "focus-up",
        label: "Focus up",
        key: "K",
        shift: false,
    },
    Action {
        id: "focus-right",
        label: "Focus right",
        key: "L",
        shift: false,
    },
    Action {
        id: "swap-left",
        label: "Swap left",
        key: "H",
        shift: true,
    },
    Action {
        id: "swap-down",
        label: "Swap down",
        key: "J",
        shift: true,
    },
    Action {
        id: "swap-up",
        label: "Swap up",
        key: "K",
        shift: true,
    },
    Action {
        id: "swap-right",
        label: "Swap right",
        key: "L",
        shift: true,
    },
    Action {
        id: "shrink-main-pane",
        label: "Shrink main pane",
        key: "[",
        shift: false,
    },
    Action {
        id: "grow-main-pane",
        label: "Grow main pane",
        key: "]",
        shift: false,
    },
    Action {
        id: "focus-left-arrow",
        label: "Focus left (arrow)",
        key: "Left",
        shift: false,
    },
    Action {
        id: "focus-down-arrow",
        label: "Focus down (arrow)",
        key: "Down",
        shift: false,
    },
    Action {
        id: "focus-up-arrow",
        label: "Focus up (arrow)",
        key: "Up",
        shift: false,
    },
    Action {
        id: "focus-right-arrow",
        label: "Focus right (arrow)",
        key: "Right",
        shift: false,
    },
    Action {
        id: "swap-left-arrow",
        label: "Swap left (arrow)",
        key: "Left",
        shift: true,
    },
    Action {
        id: "swap-down-arrow",
        label: "Swap down (arrow)",
        key: "Down",
        shift: true,
    },
    Action {
        id: "swap-up-arrow",
        label: "Swap up (arrow)",
        key: "Up",
        shift: true,
    },
    Action {
        id: "swap-right-arrow",
        label: "Swap right (arrow)",
        key: "Right",
        shift: true,
    },
    Action {
        id: "less-opaque",
        label: "Less opaque",
        key: ",",
        shift: false,
    },
    Action {
        id: "more-opaque",
        label: "More opaque",
        key: ".",
        shift: false,
    },
    Action {
        id: "restore-window-opacity",
        label: "Restore window opacity",
        key: "Backspace",
        shift: false,
    },
    Action {
        id: "restore-all-window-opacity",
        label: "Restore all window opacity",
        key: "Backspace",
        shift: true,
    },
    Action {
        id: "workspace-1",
        label: "Workspace 1",
        key: "1",
        shift: false,
    },
    Action {
        id: "workspace-2",
        label: "Workspace 2",
        key: "2",
        shift: false,
    },
    Action {
        id: "workspace-3",
        label: "Workspace 3",
        key: "3",
        shift: false,
    },
    Action {
        id: "workspace-4",
        label: "Workspace 4",
        key: "4",
        shift: false,
    },
    Action {
        id: "make-main-pane",
        label: "Make main pane",
        key: "M",
        shift: false,
    },
    Action {
        id: "cycle-layout",
        label: "Cycle layout",
        key: "V",
        shift: false,
    },
    Action {
        id: "move-to-previous-monitor",
        label: "Move to previous monitor",
        key: "PageUp",
        shift: false,
    },
    Action {
        id: "move-to-next-monitor",
        label: "Move to next monitor",
        key: "PageDown",
        shift: false,
    },
    Action {
        id: "open-windows",
        label: "Open windows",
        key: "W",
        shift: false,
    },
    Action {
        id: "rebalance-tiles",
        label: "Rebalance tiles",
        key: "R",
        shift: false,
    },
    Action {
        id: "flip-split",
        label: "Flip dwindle split",
        key: "S",
        shift: false,
    },
    Action {
        id: "move-monitor-left",
        label: "Move window to left monitor",
        key: "Ctrl+Win+Left",
        shift: true,
    },
    Action {
        id: "move-monitor-right",
        label: "Move window to right monitor",
        key: "Ctrl+Win+Right",
        shift: true,
    },
    Action {
        id: "move-monitor-up",
        label: "Move window to upper monitor",
        key: "Ctrl+Win+Up",
        shift: true,
    },
    Action {
        id: "move-monitor-down",
        label: "Move window to lower monitor",
        key: "Ctrl+Win+Down",
        shift: true,
    },
    Action {
        id: "close-window",
        label: "Close focused window",
        key: "C",
        shift: false,
    },
    Action {
        id: "maximize-window",
        label: "Maximize or restore window",
        key: "X",
        shift: false,
    },
    Action {
        id: "resize-narrower",
        label: "Resize window narrower",
        key: "Ctrl+Win+H",
        shift: false,
    },
    Action {
        id: "resize-wider",
        label: "Resize window wider",
        key: "Ctrl+Win+L",
        shift: false,
    },
    Action {
        id: "resize-shorter",
        label: "Resize window shorter",
        key: "Ctrl+Win+K",
        shift: false,
    },
    Action {
        id: "resize-taller",
        label: "Resize window taller",
        key: "Ctrl+Win+J",
        shift: false,
    },
    Action {
        id: "opacity-85",
        label: "Toggle 85% opacity",
        key: "=",
        shift: false,
    },
    Action {
        id: "opacity-90",
        label: "Toggle 90% opacity",
        key: "=",
        shift: false,
    },
    Action {
        id: "opacity-95",
        label: "Toggle 95% opacity",
        key: "=",
        shift: false,
    },
    Action {
        id: "opacity-98",
        label: "Toggle 98% opacity",
        key: "=",
        shift: false,
    },
    Action {
        id: "assign-scratchpad",
        label: "Assign focused window as scratchpad",
        key: "Ctrl+Win+P",
        shift: false,
    },
    Action {
        id: "toggle-scratchpad",
        label: "Toggle scratchpad",
        key: "P",
        shift: false,
    },
    Action {
        id: "clipboard-history",
        label: "Windows clipboard history",
        key: "Win+V",
        shift: true,
    },
    Action {
        id: "screen-capture",
        label: "Windows screen capture",
        key: "Win+S",
        shift: true,
    },
    Action {
        id: "pop-window",
        label: "Pop window out (float and keep on top)",
        key: "O",
        shift: false,
    },
];
// Keep the existing workspace-cycle WM_HOTKEY slot (38) stable for recovery/tests.
pub fn action_slot(index: usize) -> usize {
    if index >= 38 {
        index + 1
    } else {
        index
    }
}
pub fn chord(
    index: usize,
    modifiers: &str,
    overrides: &std::collections::BTreeMap<String, String>,
) -> String {
    let action = &ACTIONS[index];
    if let Some(value) = overrides.get(action.id) {
        return value.clone();
    }
    let prefix = match modifiers {
        "Alt" => "Alt",
        "CtrlAlt" => "Ctrl+Alt",
        _ => "Win+Alt",
    };
    if let Some(full) = action.key.strip_prefix('=') {
        return full.into();
    }
    let raw = format!(
        "{prefix}+{}{}",
        if action.shift { "Shift+" } else { "" },
        action.key
    );
    let mut seen = std::collections::HashSet::new();
    raw.split('+')
        .filter(|part| seen.insert(*part))
        .collect::<Vec<_>>()
        .join("+")
}
pub fn validate_overrides(
    overrides: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    for (id, value) in overrides {
        if !ACTIONS.iter().any(|action| action.id == id) {
            return Err(format!("Unknown shortcut action: {id}"));
        }
        parse(value)?;
    }
    Ok(())
}
pub fn conflict(
    binding: Binding,
    except: usize,
    config: &crate::config::Config,
) -> Option<&'static str> {
    for (index, action) in ACTIONS.iter().enumerate() {
        if index != except
            && parse(&chord(index, &config.modifiers, &config.shortcut_overrides))
                .ok()
                .flatten()
                == Some(binding)
        {
            return Some(action.label);
        }
    }
    if except != ACTIONS.len()
        && parse(&config.cycle_workspace_shortcut).ok().flatten() == Some(binding)
    {
        return Some("Cycle next workspace");
    }
    None
}

pub fn parse(text: &str) -> Result<Option<Binding>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    if text.len() > 80 {
        return Err("Shortcut is too long.".into());
    }
    let parts: Vec<_> = text.split('+').map(str::trim).collect();
    let mut modifiers = 0;
    for part in &parts[..parts.len() - 1] {
        let bit = match part.to_ascii_lowercase().as_str() {
            "alt" => 1,
            "ctrl" | "control" => 2,
            "shift" => 4,
            "win" | "super" => 8,
            _ => return Err("Use Ctrl, Alt, Shift or Win before the key.".into()),
        };
        if modifiers & bit != 0 {
            return Err("A shortcut modifier was repeated.".into());
        }
        modifiers |= bit;
    }
    if modifiers & (1 | 2 | 8) == 0 {
        return Err("Include Ctrl, Alt or Win so normal typing stays available.".into());
    }
    let name = parts.last().unwrap().to_ascii_uppercase();
    let key = match name.as_str() {
        "SPACE" => 0x20,
        "TAB" => 0x09,
        "ENTER" => 0x0d,
        "ESCAPE" | "ESC" => 0x1b,
        "BACKSPACE" => 0x08,
        "PAGEUP" | "PGUP" => 0x21,
        "PAGEDOWN" | "PGDN" => 0x22,
        "/" | "SLASH" => 0xbf,
        "[" | "LEFTBRACKET" => 0xdb,
        "]" | "RIGHTBRACKET" => 0xdd,
        "," | "COMMA" => 0xbc,
        "." | "PERIOD" => 0xbe,
        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,
        _ if name.len() == 1 && name.as_bytes()[0].is_ascii_alphanumeric() => {
            name.as_bytes()[0] as u32
        }
        _ => {
            let number = name
                .strip_prefix('F')
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|n| (1..=24).contains(n))
                .ok_or("Use a letter, digit, F1–F24, Space, Tab, Enter or arrow key.")?;
            if number == 12 {
                return Err(
                    "Windows reserves F12 for debuggers. Choose F11 or another key.".into(),
                );
            }
            0x70 + number - 1
        }
    };
    Ok(Some(Binding { modifiers, key }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_unique_and_overrides_remain_independent() {
        assert_eq!(action_slot(37), 37);
        assert_eq!(action_slot(38), 39);
        for modifier in ["Alt", "CtrlAlt", "WinAlt"] {
            let mut seen = std::collections::HashSet::new();
            for (i, action) in ACTIONS.iter().enumerate() {
                if action.key.starts_with('=') && modifier != "CtrlAlt" {
                    continue;
                }
                assert!(!action.label.is_empty());
                // Opacity toggles ship unassigned so they never collide with AutoHotkey.
                let Some(b) = parse(&chord(i, modifier, &Default::default())).unwrap() else {
                    assert!(action.id.starts_with("opacity-"), "{}", action.id);
                    continue;
                };
                assert!(
                    seen.insert((b.modifiers, b.key)),
                    "{modifier}: {}",
                    action.id
                );
            }
        }
        let mut config = crate::config::Config::default();
        config
            .shortcut_overrides
            .insert("launcher".into(), "Win+Space".into());
        assert_eq!(chord(1, "Alt", &config.shortcut_overrides), "Win+Space");
        assert_eq!(
            conflict(parse("Ctrl+Alt+T").unwrap().unwrap(), 1, &config),
            Some("Toggle tiling")
        );
    }
    #[test]
    fn supports_editing_disabling_and_rejects_unsafe_or_reserved_keys() {
        assert_eq!(
            parse(DEFAULT_CYCLE).unwrap(),
            Some(Binding {
                modifiers: 3,
                key: 0x7a
            })
        );
        assert_eq!(
            parse(" Win + Shift + a ").unwrap(),
            Some(Binding {
                modifiers: 12,
                key: 65
            })
        );
        assert_eq!(parse(" ").unwrap(), None);
        for invalid in [
            "Ctrl+Alt+F12",
            "F11",
            "Shift+A",
            "Ctrl+Ctrl+A",
            "Ctrl++A",
            "Alt+F25",
            "Ctrl+Escape+T",
        ] {
            assert!(parse(invalid).is_err(), "{invalid}");
        }
    }
}
