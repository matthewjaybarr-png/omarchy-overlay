//! Text commands shared by config\menu.json and config\binds.conf.
//! Every command names a fixed Omarchy Overlay function. Only `launch` starts an arbitrary
//! program, and only one stored inside Omarchy Overlay's own config folder.

/// Built-in commands that need no argument.
pub const BUILTINS: &[(&str, &str)] = &[
    ("apps", "Open the app picker"),
    ("menu", "Open the Omarchy Overlay menu"),
    ("keybinds", "Show all keybinds"),
    ("open-windows", "Search open windows"),
    ("window-panel", "Compact panel for the selected window"),
    ("reload-config", "Reload config files"),
    ("restart", "Restart Omarchy Overlay"),
    ("exit", "Restore the desktop and exit Omarchy Overlay"),
    ("pause-keys", "Give all keys back to Windows until resumed"),
    ("toggle-bar", "Show or hide the top bar"),
    ("keep-awake", "Keep the screen awake"),
    ("lock", "Lock the PC"),
    ("sleep", "Sleep"),
    ("sign-out", "Sign out of Windows"),
    ("restart-pc", "Restart the PC"),
    ("shutdown", "Shut down the PC"),
    ("task-view", "Windows Task View"),
    ("notifications", "Windows notifications"),
    ("quick-settings", "Windows Quick Settings"),
    ("clipboard", "Windows clipboard history"),
    ("emoji", "Windows emoji panel"),
    ("capture", "Screenshot or screen recording (Snipping Tool)"),
    ("copy", "Universal copy"),
    ("paste", "Universal paste"),
    ("cut", "Universal cut"),
    ("select-all", "Select all"),
    ("minimize-window", "Minimize the focused window"),
    ("next-desktop", "Next workspace"),
    ("former-desktop", "Former workspace"),
    ("previous-desktop", "Previous workspace"),
    (
        "move-to-desktop",
        "Move a window to another workspace (Task View)",
    ),
    ("apply-windows-theme", "Apply the theme to Windows"),
    ("next-background", "Next theme background"),
    (
        "restore-windows-appearance",
        "Restore the original Windows appearance",
    ),
    (
        "update-profile",
        "Update the active profile from current settings",
    ),
    ("guide", "Open the Omarchy Overlay guide"),
    ("log", "Open the Omarchy Overlay log"),
    ("config-folder", "Open the config folder"),
    ("releases-folder", "Show where Omarchy Overlay releases are kept"),
    (
        "restore-desktop",
        "Restore every window to its original place",
    ),
    ("reset-bar", "Reset the top bar to its defaults"),
    (
        "reset-settings",
        "Reset all settings to defaults (saved profiles are kept)",
    ),
];

/// Small settings pages. Each is drawn as a list inside the menu.
pub const PAGES: &[(&str, &str)] = &[
    ("theme", "Theme"),
    ("font", "Font"),
    ("background", "Background"),
    ("gaps", "Gaps"),
    ("main-size", "Main pane size"),
    ("layout", "Layout"),
    ("columns", "Scrolling columns"),
    ("opacity", "Window opacity"),
    ("inactive-opacity", "Inactive window opacity"),
    ("materials", "Blur and materials"),
    ("app-materials", "Material for app windows"),
    ("menu-materials", "Material for Omarchy Overlay menus"),
    ("animations", "Animations"),
    ("focus-outline", "Focus outline"),
    ("osd", "On-screen messages"),
    ("start-tiling", "Start with tiling on"),
    ("app-rules", "App rules"),
    ("default-rule", "Default for all other apps"),
    ("add-rule", "Add rule for an open window"),
    ("always-float", "Always-float list"),
    (
        "always-float-add",
        "Add an open window to the always-float list",
    ),
    ("bar", "Top bar"),
    ("bar-items", "Bar items"),
    ("bar-style", "Icons or text"),
    ("bar-left", "Left section"),
    ("bar-center", "Center section"),
    ("bar-right", "Right section"),
    ("bar-module", "Bar module"),
    ("bar-add", "Add a module"),
    ("clock", "Clock"),
    ("bar-height", "Bar height"),
    ("bar-opacity", "Bar opacity"),
    ("workspace-layouts", "Layouts by workspace"),
    ("keyboard", "Keybinds"),
    ("conflicts", "Keyboard conflicts"),
    ("keymap", "Keymap"),
    ("tap-super", "Tapping Super on its own"),
    ("taskbar", "Windows taskbar"),
    ("profiles", "Saved profiles"),
    ("windows-theme", "Apply theme to Windows"),
    ("windows-checklist", "Apps Omarchy Overlay cannot theme"),
    ("webapps", "Web apps"),
    ("install-webapp", "Install a web app"),
    ("remove-apps", "Remove from the app picker"),
    ("custom-apps", "Custom apps"),
    ("custom-app", "Custom app"),
    ("app-new", "Add by typing a path"),
    ("app-name", "Name"),
    ("app-target", "File or folder path"),
    ("app-args", "Arguments"),
    ("app-folder", "Working folder"),
    ("version", "Omarchy Overlay version"),
    ("terminal-colors", "Windows Terminal colors"),
    ("config-errors", "Config file problems"),
];

/// Work programs a command may open by role. The role follows the user's choice of app.
pub const APPS: &[(&str, &str)] = &[
    ("terminal", "Terminal"),
    ("browser", "Browser"),
    ("browser-private", "Browser (InPrivate)"),
    ("files", "File manager"),
    ("explorer", "File Explorer"),
    ("email", "Email"),
    ("new-email", "New email"),
    ("calendar", "Calendar"),
    ("teams", "Teams"),
    ("editor", "Editor"),
    ("calculator", "Calculator"),
    ("activity", "Activity (Task Manager)"),
    ("music", "Music"),
];

pub const LAYOUTS: &[(&str, &str)] = &[
    ("main-stack", "Main and stack"),
    ("columns", "Columns"),
    ("rows", "Rows"),
    ("grid", "Grid"),
    ("dwindle", "Dwindle"),
    ("scrolling", "Scrolling"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// A shortcut action from shortcuts::ACTIONS, by index.
    Action(usize),
    Builtin(&'static str),
    /// Opens the menu at the named list, for example `menu system`.
    Menu(String),
    Page(&'static str),
    Workspace(usize),
    /// Moving windows between desktops is not offered; this opens Task View instead.
    MoveToWorkspace(usize),
    Layout(&'static str),
    Theme(String),
    /// A Windows Settings page, `settings sound` opens ms-settings:sound.
    Settings(String),
    App(&'static str),
    WebApp(String),
    /// A program or script inside the config folder, run as the ordinary user.
    Launch {
        program: String,
        arguments: String,
    },
}

fn plain(text: &str, max: usize) -> bool {
    !text.is_empty()
        && text.len() <= max
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub fn parse(text: &str) -> Result<Command, String> {
    let text = text.trim();
    let (name, argument) = text
        .split_once(char::is_whitespace)
        .map_or((text, ""), |(n, a)| (n, a.trim()));
    let name = name.to_ascii_lowercase();
    let need = |what: &str| -> Result<(), String> {
        if argument.is_empty() {
            Err(format!("\"{name}\" needs {what}."))
        } else {
            Ok(())
        }
    };
    match name.as_str() {
        "" => Err("The command is empty.".into()),
        "action" => {
            need("a shortcut action name")?;
            crate::shortcuts::ACTIONS
                .iter()
                .position(|a| a.id.eq_ignore_ascii_case(argument))
                .map(Command::Action)
                .ok_or_else(|| format!("Unknown action \"{argument}\"."))
        }
        "menu" if !argument.is_empty() => {
            if plain(argument, 40) {
                Ok(Command::Menu(argument.to_ascii_lowercase()))
            } else {
                Err(format!("Unknown menu \"{argument}\"."))
            }
        }
        "page" => {
            need("a page name")?;
            PAGES
                .iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(argument))
                .map(|(id, _)| Command::Page(id))
                .ok_or_else(|| format!("Unknown page \"{argument}\"."))
        }
        "workspace" | "move-to-workspace" => {
            need("a number from 1 to 9")?;
            let number = argument
                .parse::<usize>()
                .ok()
                .filter(|n| (1..=9).contains(n))
                .ok_or("Workspaces are numbered 1 to 9.")?;
            Ok(if name == "workspace" {
                Command::Workspace(number)
            } else {
                Command::MoveToWorkspace(number)
            })
        }
        "layout" => {
            need("a layout name")?;
            LAYOUTS
                .iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(argument))
                .map(|(id, _)| Command::Layout(id))
                .ok_or_else(|| format!("Unknown layout \"{argument}\"."))
        }
        "theme" => {
            need("a theme name")?;
            if plain(argument, 64) {
                Ok(Command::Theme(argument.to_ascii_lowercase()))
            } else {
                Err("Theme names use letters, digits and dashes.".into())
            }
        }
        "settings" => {
            // An empty argument opens the Settings home page.
            if argument.is_empty() || plain(argument, 64) {
                Ok(Command::Settings(argument.to_ascii_lowercase()))
            } else {
                Err(format!(
                    "\"{argument}\" is not a Windows Settings page name."
                ))
            }
        }
        "app" => {
            need("an app role such as terminal or browser")?;
            APPS.iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(argument))
                .map(|(id, _)| Command::App(id))
                .ok_or_else(|| format!("Unknown app role \"{argument}\"."))
        }
        "webapp" => {
            need("the web app's name")?;
            if argument.len() <= 80 && !argument.chars().any(char::is_control) {
                Ok(Command::WebApp(argument.into()))
            } else {
                Err("The web app name is too long.".into())
            }
        }
        "launch" => {
            need("a program inside the config folder")?;
            let (program, arguments) = split_program(argument)?;
            // Both slash kinds separate folders on Windows; check the parts by hand
            // so the rule is identical when the tests run elsewhere.
            if program.contains(':')
                || program
                    .split(['\\', '/'])
                    .any(|part| matches!(part.trim(), "" | "." | ".."))
            {
                return Err(
                    "launch only runs programs stored inside the config folder; use a path like tools\\example.cmd."
                        .into(),
                );
            }
            Ok(Command::Launch { program, arguments })
        }
        _ => {
            let builtin = BUILTINS
                .iter()
                .find(|(id, _)| *id == name)
                .map(|(id, _)| Command::Builtin(id))
                .ok_or_else(|| format!("Unknown command \"{name}\"."))?;
            if !argument.is_empty() {
                return Err(format!("\"{name}\" does not take anything after it."));
            }
            Ok(builtin)
        }
    }
}

/// Splits `"tools\my script.cmd" --flag` or `tools\run.cmd --flag` into program and arguments.
fn split_program(text: &str) -> Result<(String, String), String> {
    if let Some(rest) = text.strip_prefix('"') {
        let (program, arguments) = rest
            .split_once('"')
            .ok_or("A quoted program name is missing its closing quote.")?;
        return Ok((program.into(), arguments.trim().into()));
    }
    let (program, arguments) = text
        .split_once(char::is_whitespace)
        .map_or((text, ""), |(p, a)| (p, a.trim()));
    Ok((program.into(), arguments.into()))
}

impl Command {
    /// True when the command opens a Windows screen rather than a Omarchy Overlay one.
    pub fn windows(&self) -> bool {
        match self {
            Self::Settings(_) | Self::MoveToWorkspace(_) => true,
            Self::Builtin(name) => matches!(
                *name,
                "task-view"
                    | "notifications"
                    | "quick-settings"
                    | "clipboard"
                    | "emoji"
                    | "capture"
                    | "move-to-desktop"
            ),
            _ => false,
        }
    }
    /// Commands that act again while their key is held down.
    pub fn repeats(&self) -> bool {
        matches!(self, Self::Action(index) if {
            let id = crate::shortcuts::ACTIONS[*index].id;
            id.starts_with("resize-") || id.ends_with("-opaque") || id.contains("main-pane")
        })
    }
    /// Commands that only make sense while tiling is running.
    pub fn tiling(&self) -> bool {
        match self {
            Self::Action(index) => {
                let id = crate::shortcuts::ACTIONS[*index].id;
                id.starts_with("focus-")
                    || id.starts_with("swap-")
                    || id.starts_with("resize-")
                    || id.starts_with("move-monitor-")
                    || matches!(
                        id,
                        "float"
                            | "make-main-pane"
                            | "cycle-layout"
                            | "rebalance-tiles"
                            | "flip-split"
                            | "shrink-main-pane"
                            | "grow-main-pane"
                    )
            }
            Self::Layout(_) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_parse_to_fixed_functions_only() {
        assert_eq!(parse("apps"), Ok(Command::Builtin("apps")));
        assert_eq!(parse(" menu system "), Ok(Command::Menu("system".into())));
        assert_eq!(parse("menu"), Ok(Command::Builtin("menu")));
        assert_eq!(parse("workspace 9"), Ok(Command::Workspace(9)));
        assert!(parse("workspace 10").is_err());
        assert_eq!(
            parse("settings sound"),
            Ok(Command::Settings("sound".into()))
        );
        assert!(parse("settings sound;calc").is_err());
        assert_eq!(parse("app files"), Ok(Command::App("files")));
        assert_eq!(
            parse("webapp ChatGPT"),
            Ok(Command::WebApp("ChatGPT".into()))
        );
        let close = crate::shortcuts::ACTIONS
            .iter()
            .position(|a| a.id == "close-window")
            .unwrap();
        assert_eq!(parse("action close-window"), Ok(Command::Action(close)));
        assert!(parse("action format-disk").is_err());
        assert!(parse("lock now").is_err());
        assert!(parse("cmd /c del").is_err());
        assert_eq!(
            parse("launch \"tools\\my script.cmd\" one two"),
            Ok(Command::Launch {
                program: "tools\\my script.cmd".into(),
                arguments: "one two".into()
            })
        );
        assert!(parse("launch C:\\Windows\\System32\\cmd.exe").is_err());
        assert!(parse("launch ..\\OmarchyOverlay.exe").is_err());
        assert!(parse("launch \\\\server\\share\\x.cmd").is_err());
        assert!(parse("settings").unwrap().windows());
        assert!(parse("action resize-wider").unwrap().repeats());
        assert!(parse("action focus-left").unwrap().tiling());
        assert!(!parse("apps").unwrap().tiling());
    }
}
