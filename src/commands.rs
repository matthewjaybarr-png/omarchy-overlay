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
    ("record-screen", "Screen recording (Snipping Tool)"),
    ("extract-text", "Copy text from the screen (Snipping Tool)"),
    ("color-picker", "Pick a color from the screen and copy its hex value"),
    ("console", "Drop-down console: show, hide or start it"),
    ("hibernate", "Hibernate the PC"),
    ("toggle-laptop-display", "Laptop screen off, or back on"),
    ("toggle-mirror", "Mirror the screens, or extend them again"),
    ("restart-to-firmware", "Restart into the firmware (BIOS/UEFI) setup (Admin edition)"),
    ("voice-typing", "Windows voice typing (dictation)"),
    ("close-all", "Close every window on this workspace"),
    ("full-screen-desktop", "Hide the bar and the gaps, or bring them back"),
    ("single-window-square", "Keep a lone window square, or let it fill the screen"),
    ("pseudo-tile", "Keep the window's own size inside its tile, or fill the tile"),
    ("group-window-1", "Show window 1 of the group"),
    ("group-window-2", "Show window 2 of the group"),
    ("group-window-3", "Show window 3 of the group"),
    ("group-window-4", "Show window 4 of the group"),
    ("group-window-5", "Show window 5 of the group"),
    ("toggle-group", "Make the window a group, or ungroup it"),
    ("leave-group", "Take the window out of its group"),
    ("move-into-group-left", "Move the window into the group on the left"),
    ("move-into-group-right", "Move the window into the group on the right"),
    ("move-into-group-up", "Move the window into the group above"),
    ("move-into-group-down", "Move the window into the group below"),
    ("next-in-group", "Next window in the group"),
    ("previous-in-group", "Previous window in the group"),
    ("show-reminders", "Show the reminders that are set"),
    ("clear-reminders", "Clear all reminders"),
    ("copy", "Universal copy"),
    ("paste", "Universal paste"),
    ("cut", "Universal cut"),
    ("select-all", "Select all"),
    ("minimize-window", "Minimize the focused window"),
    ("volume-up", "Volume up"),
    ("volume-down", "Volume down"),
    ("mute", "Sound on or off"),
    ("mic-mute", "Microphone on or off"),
    ("zoom-in", "Zoom in (Windows Magnifier)"),
    ("zoom-reset", "Reset zoom (close Windows Magnifier)"),
    ("cycle-layout-back", "Cycle workspace layout back"),
    ("full-screen", "Full screen over the bar, or back"),
    ("save-width", "Save window width"),
    ("restore-width", "Restore window width"),
    ("focus-next-monitor", "Focus on next monitor"),
    ("focus-previous-monitor", "Focus on previous monitor"),
    ("move-workspace-left", "Move workspace to left monitor"),
    ("move-workspace-right", "Move workspace to right monitor"),
    ("move-workspace-up", "Move workspace to up monitor"),
    ("move-workspace-down", "Move workspace to down monitor"),
    ("brightness-up", "Monitor brightness up (external monitors)"),
    ("brightness-down", "Monitor brightness down (external monitors)"),
    ("toggle-gaps", "Turn the gaps between windows off or back on"),
    ("show-time", "Show the time and date"),
    ("show-battery", "Show the battery level"),
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
    ("transparency", "Window transparency"),
    ("idle", "Screensaver and idle lock"),
    ("bar-position", "Bar position"),
    ("idle-screensaver", "Screensaver"),
    ("idle-lock", "Lock when idle"),
    ("corners", "Window corners"),
    ("windows-tweaks", "Windows behavior"),
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
    ("vscode-colors", "VS Code colors"),
    ("obsidian-colors", "Obsidian colors"),
    ("browser-colors", "Browser colors"),
    ("lock-screen", "Lock screen picture"),
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
    ("centered", "Centered main"),
    ("monocle", "Monocle"),
    ("floating", "Floating"),
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
    MoveSilently(usize),
    Layout(&'static str),
    /// Resizes the focused tile by `pixels`, wider/taller when positive, `resize width -100`.
    Resize { width: bool, pixels: i32 },
    /// Shows a reminder after this many minutes, `remind 15`.
    Remind(u32, String),
    Theme(String),
    /// Installs an Omarchy theme from its git repository, `theme-install owner/repo`.
    ThemeInstall(String),
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
        "workspace" | "move-to-workspace" | "move-to-workspace-silent" => {
            need("a number from 1 to 10")?;
            let number = argument
                .parse::<usize>()
                .ok()
                .filter(|n| (1..=10).contains(n))
                .ok_or("Workspaces are numbered 1 to 10.")?;
            Ok(match name.as_str() {
                "workspace" => Command::Workspace(number),
                "move-to-workspace" => Command::MoveToWorkspace(number),
                _ => Command::MoveSilently(number),
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
        "resize" => {
            need("width or height and a number of pixels")?;
            let (side, pixels) = argument.split_once(char::is_whitespace).unwrap_or((argument, ""));
            let width = match side.to_ascii_lowercase().as_str() {
                "width" => true,
                "height" => false,
                _ => return Err("resize takes width or height, as in resize width -100.".into()),
            };
            let pixels = pixels
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|n| *n != 0 && (-2000..=2000).contains(n))
                .ok_or("resize needs a number of pixels from -2000 to 2000, as in resize width -100.")?;
            Ok(Command::Resize { width, pixels })
        }
        "remind" => {
            need("a number of minutes")?;
            let (minutes, message) = argument.split_once(char::is_whitespace).unwrap_or((argument, ""));
            let message = message.trim();
            if message.chars().count() > 200 || message.chars().any(char::is_control) {
                return Err("A reminder message is one line of up to 200 characters.".into());
            }
            minutes
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=1440).contains(n))
                .map(|n| Command::Remind(n, message.to_string()))
                .ok_or_else(|| "remind needs minutes from 1 to 1440, as in remind 15 or remind 15 Stretch.".into())
        }
        "theme" => {
            need("a theme name")?;
            if plain(argument, 64) {
                Ok(Command::Theme(argument.to_ascii_lowercase()))
            } else {
                Err("Theme names use letters, digits and dashes.".into())
            }
        }
        "theme-install" => {
            need("a theme repository, as in theme-install bjarneo/omarchy-ash-theme")?;
            crate::theme_files::theme_repo(argument)
                .map(|_| Command::ThemeInstall(argument.trim().to_string()))
                .ok_or_else(|| "theme-install takes owner/repo or an https:// git address.".into())
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
            Self::Settings(_) => true,
            Self::Builtin(name) => matches!(
                *name,
                "task-view"
                    | "notifications"
                    | "quick-settings"
                    | "clipboard"
                    | "emoji"
                    | "capture"
                    | "record-screen"
                    | "extract-text"
                    | "voice-typing"
                    | "move-to-desktop"
            ),
            _ => false,
        }
    }
    /// Commands that act again while their key is held down.
    pub fn repeats(&self) -> bool {
        matches!(self, Self::Resize { .. }) || matches!(self, Self::Action(index) if {
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
            Self::Builtin(name) => {
                *name == "cycle-layout-back"
                    || *name == "restore-width"
                    || name.starts_with("move-workspace-")
                    || name.contains("group")
            }
            Self::Resize { .. } | Self::Layout(_) | Self::Workspace(_) | Self::MoveToWorkspace(_) | Self::MoveSilently(_) => true,
            _ => false,
        }
    }
}

/// A brightness value moved by `percent` of the monitor's range, kept inside it.
/// Window corner styles; the position is the DWM corner preference value.
pub const CORNERS: &[(&str, &str)] = &[
    ("default", "As Windows draws them"),
    ("square", "Square"),
    ("round", "Round"),
    ("small", "Slightly round"),
];

pub fn brightness_step(current: u32, maximum: u32, percent: i32) -> u32 {
    (i64::from(current) + i64::from(maximum) * i64::from(percent) / 100)
        .clamp(0, i64::from(maximum)) as u32
}

#[cfg(test)]
mod tests {
    #[test]
    fn brightness_steps_stay_inside_the_monitor_range() {
        assert_eq!(super::brightness_step(50, 100, 10), 60);
        assert_eq!(super::brightness_step(95, 100, 10), 100);
        assert_eq!(super::brightness_step(5, 100, -10), 0);
        assert_eq!(super::brightness_step(30, 255, -10), 5);
    }

    use super::*;
    #[test]
    fn resize_takes_a_side_and_a_bounded_step() {
        assert_eq!(
            parse("resize width -100"),
            Ok(Command::Resize { width: true, pixels: -100 })
        );
        assert_eq!(
            parse("Resize HEIGHT 25"),
            Ok(Command::Resize { width: false, pixels: 25 })
        );
        assert_eq!(parse("remind 15"), Ok(Command::Remind(15, String::new())));
        assert_eq!(parse("remind 20  Take a break "), Ok(Command::Remind(20, "Take a break".into())));
        for bad in ["remind", "remind 0", "remind 1441", "remind soon"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        for bad in ["resize", "resize width", "resize depth 10", "resize width 0", "resize width 5000", "resize width -2147483648"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        assert!(parse("resize width 30").unwrap().repeats());
        assert!(parse("cycle-layout-back").unwrap().tiling());
    }
    #[test]
    fn commands_parse_to_fixed_functions_only() {
        assert_eq!(parse("apps"), Ok(Command::Builtin("apps")));
        assert_eq!(parse(" menu system "), Ok(Command::Menu("system".into())));
        assert_eq!(parse("menu"), Ok(Command::Builtin("menu")));
        assert_eq!(parse("workspace 9"), Ok(Command::Workspace(9)));
        assert_eq!(parse("workspace 10"), Ok(Command::Workspace(10)));
        assert!(parse("workspace 11").is_err());
        assert_eq!(parse("move-to-workspace-silent 2"), Ok(Command::MoveSilently(2)));
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
