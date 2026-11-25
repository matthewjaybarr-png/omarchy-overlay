//! Runs one command from a Super key, menu.json or the menu pages. Every command
//! names a fixed Omarchy Overlay function; `launch` only starts files inside the config
//! folder, as the ordinary user.
use super::overlay::{self, Level};
use super::*;
use crate::commands::Command;
use windows_sys::Win32::System::{Power::*, Registry::*, Shutdown::*};

/// Posted after the menu hides so a command acts on the window behind it.
pub(super) const PENDING_COMMAND: u32 = WM_APP + 72;

/// Commands that ask before they run. The power key, menu and binds all go through this.
pub(super) fn question(command: &Command) -> Option<&'static str> {
    match command {
        Command::Builtin("sleep") => Some("Put the PC to sleep?"),
        Command::Builtin("sign-out") => {
            Some("Sign out of Windows? Unsaved work in open apps may be lost.")
        }
        Command::Builtin("restart-pc") => {
            Some("Restart the PC now? Unsaved work in open apps may be lost.")
        }
        Command::Builtin("shutdown") => {
            Some("Shut down the PC now? Unsaved work in open apps may be lost.")
        }
        Command::Builtin("exit") => Some("Restore every window and exit Omarchy Overlay?"),
        Command::Builtin("reset-bar") => {
            Some("Reset the top bar? Its items, icons, height, clock and opacity go back to the defaults.")
        }
        Command::Builtin("reset-settings") => Some(
            "Reset all settings to defaults? Saved profiles and config files are kept, and a backup is saved first.",
        ),
        _ => None,
    }
}

/// Reads a text value under HKEY_CURRENT_USER. None when it is missing.
unsafe fn user_text(key: &str, name: &str) -> Option<String> {
    let key = wide(key);
    let name = wide(name);
    let mut text = [0u16; 260];
    let mut bytes = (text.len() * 2) as u32;
    if RegGetValueW(
        HKEY_CURRENT_USER,
        key.as_ptr(),
        name.as_ptr(),
        RRF_RT_REG_SZ,
        null_mut(),
        text.as_mut_ptr() as _,
        &mut bytes,
    ) != ERROR_SUCCESS
    {
        return None;
    }
    let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
    Some(String::from_utf16_lossy(&text[..end]))
}

/// The program and private-window switch of the user's default web browser.
unsafe fn browser() -> (&'static str, &'static str) {
    let id = user_text(
        "Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations\\https\\UserChoice",
        "ProgId",
    )
    .unwrap_or_default()
    .to_ascii_lowercase();
    if id.starts_with("chrome") {
        ("chrome.exe", "--incognito")
    } else if id.starts_with("firefox") {
        ("firefox.exe", "-private-window")
    } else if id.starts_with("brave") {
        ("brave.exe", "--incognito")
    } else {
        ("msedge.exe", "--inprivate")
    }
}

/// The first open window whose program is one of these names.
unsafe fn window_of(programs: &[&str]) -> Option<tasks::Task> {
    tasks::collect().into_iter().find(|t| {
        let name = process_name(t.hwnd);
        programs.iter().any(|p| name.eq_ignore_ascii_case(p))
    })
}

impl App {
    /// Runs a command. `confirmed` is true once its question has been answered Yes.
    pub(super) unsafe fn run_command(&mut self, command: &Command, confirmed: bool) {
        if !confirmed {
            if let Some(question) = question(command) {
                self.open_level(
                    Level::Confirm(Box::new(command.clone()), question.into()),
                    true,
                );
                return;
            }
        }
        match command {
            Command::Builtin("apps") => self.open_level(Level::Apps, true),
            Command::Builtin("menu") => self.open_menu("main"),
            Command::Builtin("keybinds") => self.open_level(Level::Keybinds, true),
            Command::Builtin("open-windows") => self.open_level(Level::Windows, true),
            Command::Builtin("window-panel") => {
                let target = GetForegroundWindow();
                self.open_level(Level::WindowPanel(target as isize), true);
            }
            Command::Menu(id) => self.open_menu(id),
            Command::Page(page) => self.open_level(Level::Page(page), true),
            Command::Builtin(name) => self.run_builtin(name),
            Command::Action(index) => {
                self.action(crate::shortcuts::action_slot(*index) as u32);
            }
            Command::Workspace(number) => {
                if crate::desktops::select(number.saturating_sub(1)) {
                    self.tick();
                    bar::poll();
                }
            }
            Command::MoveToWorkspace(number) => self.move_to_workspace(number - 1, false),
            Command::MoveSilently(number) => self.move_to_workspace(number - 1, true),
            Command::Layout(layout) => self.set_layout(layout),
            Command::Resize { width, pixels } => {
                self.resize_keyboard(GetForegroundWindow(), *width, *pixels)
            }
            Command::Remind(minutes, text) => self.remind(*minutes, text),
            Command::ThemeInstall(repo) => self.install_theme(repo),
            Command::Theme(name) => {
                let mut c = self.config.clone();
                c.theme = name.clone();
                self.commit(c);
            }
            Command::Settings(page) => {
                shell(Path::new(&format!("ms-settings:{page}")), "");
            }
            Command::App(role) => self.open_app(role),
            Command::WebApp(name) => self.open_webapp(name, confirmed),
            Command::Launch { program, arguments } => self.launch_from_config(program, arguments),
        }
    }

    unsafe fn run_builtin(&mut self, name: &str) {
        match name {
            "reload-config" => self.reload_config(),
            "restart" => self.restart_overlay(),
            "exit" => PostQuitMessage(0),
            "pause-keys" => {
                self.keys_paused = !self.keys_paused;
                self.restart_keys();
                self.show_feedback(if self.keys_paused {
                    "Omarchy Overlay keys paused. Every key goes to Windows."
                } else {
                    "Omarchy Overlay keys are back."
                });
            }
            "toggle-bar" => {
                let mut c = self.config.clone();
                c.show_top_bar = !c.show_top_bar;
                self.commit(c);
            }
            "keep-awake" => {
                self.keep_awake = !self.keep_awake;
                SetThreadExecutionState(if self.keep_awake {
                    ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED
                } else {
                    ES_CONTINUOUS
                });
                self.show_feedback(if self.keep_awake {
                    "Screen stays awake"
                } else {
                    "Screen may sleep again"
                });
            }
            "lock" => {
                LockWorkStation();
            }
            "toggle-laptop-display" | "toggle-mirror" => {
                let result = if name == "toggle-mirror" {
                    display_topology::toggle_mirror()
                } else {
                    display_topology::toggle_laptop()
                };
                match result {
                    Ok(text) => self.show_feedback(text),
                    Err(error) => self.show_feedback(&error),
                }
            }
            "hibernate" => {
                if !SetSuspendState(true, false, false) {
                    self.notice("Windows did not allow hibernate. It may be turned off on this PC (powercfg /hibernate on).");
                }
            }
            "voice-typing" => self.queue_system_action(17),
            "close-all" => {
                // Omarchy's close-all: every app window on this workspace is asked to close,
                // as its own X button would, so apps can still ask about unsaved work.
                for task in tasks::collect() {
                    if !self.parked.contains_key(&(task.hwnd as isize)) && self.movable(task.hwnd) {
                        PostMessageW(task.hwnd, WM_CLOSE, 0, 0);
                    }
                }
            }
            "pseudo-tile" => {
                let key = GetForegroundWindow() as isize;
                if self.order.contains(&key) {
                    if !self.pseudo.remove(&key) {
                        self.pseudo.insert(key);
                    }
                    self.tiled.clear();
                    self.dirty = true;
                    self.tick();
                }
            }
            "single-window-square" => {
                let mut c = self.config.clone();
                c.single_window_square = !c.single_window_square;
                self.show_feedback(if c.single_window_square { "A lone window stays square" } else { "A lone window fills the screen" });
                // The cached tiles don't know the option: lay every monitor out again.
                self.tiled.clear();
                self.commit(c);
            }
            "full-screen-desktop" => {
                // Omarchy's Super+Ctrl+Alt+F: no bar and no gaps, or both back.
                let mut c = self.config.clone();
                let bare = !c.show_top_bar && c.gap == 0 && c.outer_gap == 0;
                if bare {
                    c.show_top_bar = true;
                    let d = Config::default();
                    let (gap, outer) = std::mem::take(&mut self.gaps_before);
                    (c.gap, c.outer_gap) = if (gap, outer) == (0, 0) { (d.gap, d.outer_gap) } else { (gap, outer) };
                } else {
                    c.show_top_bar = false;
                    if (c.gap, c.outer_gap) != (0, 0) {
                        self.gaps_before = (std::mem::take(&mut c.gap), std::mem::take(&mut c.outer_gap));
                    }
                }
                self.commit(c);
            }
            name if name.starts_with("group-window-") => {
                if let Ok(n) = name.trim_start_matches("group-window-").parse::<usize>() {
                    self.show_group_window(GetForegroundWindow(), n.saturating_sub(1));
                }
            }
            "sleep" => {
                if !SetSuspendState(false, false, false) {
                    self.notice("Windows did not allow sleep. Use Start → Power → Sleep.");
                }
            }
            "sign-out" => {
                if ExitWindowsEx(EWX_LOGOFF, 0) == 0 {
                    self.notice("Windows did not allow sign-out. Use Start → your name → Sign out.");
                }
            }
            "restart-to-firmware" => {
                // Omarchy's "Reboot to BIOS". Windows only allows it with administrator
                // rights, so it runs here, from the elevated Admin edition, not through the
                // unelevated launch every app gets.
                if !crate::launch_unelevated::process_elevated() {
                    self.notice("Restarting into the firmware (BIOS/UEFI) setup needs the Admin edition, which runs with administrator rights.");
                } else {
                    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
                    let tool = crate::install::system_tool(&windir, "shutdown.exe");
                    // A one-second delay: shutdown.exe takes /fw with a timer, as Windows'
                    // own advanced startup does.
                    match std::process::Command::new(&tool).args(["/r", "/fw", "/t", "1"]).output() {
                        Ok(out) if out.status.success() => log("Restarting into the firmware setup."),
                        Ok(out) => {
                            let said = String::from_utf8_lossy(&out.stderr).trim().to_string()
                                + String::from_utf8_lossy(&out.stdout).trim();
                            log(&format!("{tool} /r /fw /t 1 exited with {:?}: {said}", out.status.code()));
                            self.notice(&format!("Windows did not restart into the firmware setup ({:?}). {said}", out.status.code()));
                        }
                        Err(error) => {
                            log(&format!("{tool} could not start: {error}"));
                            self.notice(&format!("shutdown.exe could not start: {error}"));
                        }
                    }
                }
            }
            "restart-pc" | "shutdown" => {
                let switch = if name == "shutdown" { "/s /t 0" } else { "/r /t 0" };
                if !shell(Path::new("shutdown.exe"), switch) {
                    self.notice("Windows did not start the shutdown. Use Start → Power.");
                }
            }
            "task-view" => self.queue_system_action(6),
            "notifications" => self.queue_system_action(4),
            "quick-settings" => self.queue_system_action(3),
            "clipboard" => self.queue_system_action(0),
            "emoji" => self.queue_system_action(5),
            "capture" => self.queue_system_action(1),
            "record-screen" => self.queue_system_action(15),
            "extract-text" => self.queue_system_action(16),
            "color-picker" => color_picker::start(self.main),
            "console" => self.toggle_console(),
            "toggle-group" => self.toggle_group(GetForegroundWindow()),
            "leave-group" => self.leave_group(GetForegroundWindow()),
            "move-into-group-left" => self.move_into_group(GetForegroundWindow(), -1, 0),
            "move-into-group-right" => self.move_into_group(GetForegroundWindow(), 1, 0),
            "move-into-group-up" => self.move_into_group(GetForegroundWindow(), 0, -1),
            "move-into-group-down" => self.move_into_group(GetForegroundWindow(), 0, 1),
            "next-in-group" => self.cycle_group(GetForegroundWindow(), 1),
            "previous-in-group" => self.cycle_group(GetForegroundWindow(), -1),
            "show-reminders" => self.show_reminders(),
            "clear-reminders" => self.clear_reminders(),
            "copy" | "paste" | "cut" | "select-all" => {
                // Like Omarchy, terminals copy and paste with Ctrl+Shift so Ctrl+C still interrupts.
                let terminal = process_name(GetForegroundWindow()).eq_ignore_ascii_case("WindowsTerminal.exe");
                self.queue_system_action(match (name, terminal) {
                    ("copy", true) => 11,
                    ("paste", true) => 12,
                    ("copy", _) => 7,
                    ("paste", _) => 8,
                    ("cut", _) => 9,
                    _ => 10,
                });
            }
            "minimize-window" => {
                let window = GetForegroundWindow();
                if !window.is_null() {
                    ShowWindow(window, SW_MINIMIZE);
                }
            }
            "volume-up" => {
                native_status::adjust_volume(5);
                self.feedback_message(0);
            }
            "volume-down" => {
                native_status::adjust_volume(-5);
                self.feedback_message(0);
            }
            "mute" => {
                native_status::toggle_mute();
                self.feedback_message(0);
            }
            "mic-mute" => match native_status::toggle_mic_mute() {
                Some(true) => self.show_card(Some(osd::MIC_OFF), "Microphone muted", None),
                Some(false) => self.show_card(Some(osd::MIC), "Microphone on", None),
                None => self.show_feedback("No microphone found"),
            },
            "zoom-in" => self.queue_system_action(13),
            "zoom-reset" => self.queue_system_action(14),
            "cycle-layout-back" => self.cycle_layout(true),
            "full-screen" => self.toggle_full_screen(GetForegroundWindow()),
            "save-width" => self.save_width(GetForegroundWindow()),
            "restore-width" => self.restore_width(GetForegroundWindow()),
            "focus-next-monitor" => self.focus_monitor(1),
            "focus-previous-monitor" => self.focus_monitor(-1),
            "move-workspace-left" => self.move_workspace_to_monitor(-1, 0),
            "move-workspace-right" => self.move_workspace_to_monitor(1, 0),
            "move-workspace-up" => self.move_workspace_to_monitor(0, -1),
            "move-workspace-down" => self.move_workspace_to_monitor(0, 1),
            "brightness-up" => brightness::adjust(10, self.main),
            "brightness-down" => brightness::adjust(-10, self.main),
            "toggle-gaps" => {
                // As Omarchy's toggle: the gaps at the screen edges go too, not only
                // those between windows, so the tiles meet the edges and each other.
                let mut c = self.config.clone();
                if c.gap == 0 && c.outer_gap == 0 {
                    let d = Config::default();
                    let (gap, outer) = std::mem::take(&mut self.gaps_before);
                    (c.gap, c.outer_gap) = if (gap, outer) == (0, 0) { (d.gap, d.outer_gap) } else { (gap, outer) };
                    self.show_feedback("Gaps on");
                } else {
                    self.gaps_before = (std::mem::take(&mut c.gap), std::mem::take(&mut c.outer_gap));
                    self.show_feedback("Gaps off");
                }
                self.commit(c);
            }
            "show-time" => {
                let mut t: SYSTEMTIME = zeroed();
                windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t);
                let format = if self.config.bar.clock_format.ends_with("-12h") {
                    "day-time-12h"
                } else {
                    "day-time"
                };
                self.show_feedback(&crate::bar_model::clock_label(
                    format,
                    [t.wYear, t.wMonth, t.wDay],
                    t.wDayOfWeek,
                    t.wHour,
                    t.wMinute,
                ));
            }
            "show-battery" => {
                use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
                let mut power: SYSTEM_POWER_STATUS = zeroed();
                let known = GetSystemPowerStatus(&mut power) != 0;
                self.show_feedback(&if !known || power.BatteryLifePercent > 100 {
                    "This PC has no battery".to_string()
                } else if power.ACLineStatus == 1 {
                    format!("Battery {}%, plugged in", power.BatteryLifePercent)
                } else {
                    format!("Battery {}%", power.BatteryLifePercent)
                });
            }
            "former-desktop" => {
                if crate::desktops::select_former() {
                    self.tick();
                    bar::poll();
                } else {
                    self.show_feedback("No former workspace yet.");
                }
            }
            "next-desktop" | "previous-desktop" => {
                let moved = match crate::desktops::snapshot() {
                    Some(s) if s.ids.len() > 1 && name == "previous-desktop" => {
                        crate::desktops::select((s.current + s.ids.len() - 1) % s.ids.len())
                    }
                    _ => crate::desktops::cycle_next(),
                };
                if moved {
                    self.tick();
                    bar::poll();
                }
            }
            "move-to-desktop" => crate::desktops::task_view(),
            "apply-windows-theme" => {
                let result = appearance_effects::with_effects(|| windows_personalization::apply(&self.directory, theme::current().dark));
                self.show_feedback(&match result {
                    Ok(()) => "Windows light or dark mode now follows the theme. Restore puts it back.".into(),
                    Err(error) => error,
                });
            }
            "next-background" => {
                match appearance_effects::cycle_background(&self.directory, &self.config.theme, false) {
                    Ok(true) => {}
                    Ok(false) => self.show_feedback("This theme has no backgrounds folder."),
                    Err(error) => self.show_feedback(&error),
                }
            }
            "restore-windows-appearance" => match appearance_effects::restore(&self.directory) {
                Ok(()) => {
                    // Asked for the original look: Terminal and VS Code stay unthemed.
                    self.config.terminal_colors = false;
                    self.config.vscode_colors = false;
                    self.config.obsidian_colors = false;
                    self.config.browser_colors = false;
                    self.config.lock_screen = false;
                    self.config.install_fonts = false;
                    self.save_config();
                    self.show_feedback("Original Windows appearance restored.")
                }
                Err(error) => self.notice(&error),
            },
            "update-profile" => self.update_profile(),
            "guide" => {
                let guide = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(|d| d.join("README.md")))
                    .unwrap_or_else(|| PathBuf::from("README.md"));
                if !shell(Path::new("notepad.exe"), &format!("\"{}\"", guide.display())) {
                    self.notice(&format!("The guide could not be opened: {}", guide.display()));
                }
            }
            "log" => {
                let path = data_directory().join("omarchy-overlay.log");
                shell(Path::new("notepad.exe"), &format!("\"{}\"", path.display()));
            }
            "config-folder" => {
                let folder = config_folder::folder();
                let _ = fs::create_dir_all(&folder);
                shell(&folder, "");
            }
            "releases-folder" => self.notice(
                "Omarchy Overlay releases are the zips you downloaded. \
                 Each zip has SHA256SUMS.txt beside it. Omarchy Overlay never downloads or installs updates.",
            ),
            "reset-bar" => {
                let mut c = self.config.clone();
                let d = Config::default();
                c.bar = d.bar;
                c.show_top_bar = d.show_top_bar;
                c.bar_opacity = d.bar_opacity;
                if self.commit(c) {
                    self.show_feedback("The top bar is back to its defaults.");
                }
            }
            "reset-settings" => match self.reset_settings() {
                Ok(()) => self.show_feedback(
                    "All settings are back to their defaults. A backup is in profile-backup.json.",
                ),
                Err(error) => self.notice(&format!("Reset failed: {error}")),
            },
            "restore-desktop" => {
                self.restore();
                if self.saved.is_empty() {
                    self.show_feedback("Every window is back in place.");
                } else {
                    self.open_level(Level::Page("restore-failed"), true);
                }
            }
            _ => {}
        }
    }

    /// Re-reads every file in the config folder. A broken file keeps its built-in version.
    pub(super) unsafe fn reload_config(&mut self) {
        let loaded = config_folder::load();
        self.binds = loaded.binds;
        self.menus = loaded.menus;
        self.webapps = loaded.webapps;
        self.config.always_float = loaded.always_float;
        self.config_errors = loaded.errors;
        let theme = self.config.theme.clone();
        self.apply_theme(if theme::palette(&theme).is_some() {
            &theme
        } else {
            "neon-ridge"
        });
        self.apply_rules();
        self.restart_keys();
        // Picks up apps installed since Omarchy Overlay started.
        self.command_menu = false;
        self.refresh_apps();
        if self.config_errors.is_empty() {
            if overlay::visible() {
                overlay::message("Config reloaded.");
            } else {
                self.show_feedback("Config reloaded.");
            }
        } else {
            self.open_level(Level::Page("config-errors"), true);
        }
    }

    /// Starts a new copy of Omarchy Overlay that waits for this one to restore and exit.
    unsafe fn restart_overlay(&mut self) {
        let Ok(exe) = std::env::current_exe() else {
            self.notice("Omarchy Overlay could not find its own program file.");
            return;
        };
        let mut arguments = format!("--restart {}", GetCurrentProcessId());
        let mut rest = std::env::args().skip(1);
        while let Some(argument) = rest.next() {
            if argument == "--restart" {
                rest.next();
                continue;
            }
            arguments.push(' ');
            arguments.push_str(&crate::launch_unelevated::quote(&argument));
        }
        // Direct, not shell_in: an elevated install must restart elevated, not as a user app.
        if shell_direct(&exe, &arguments, exe.parent()) {
            PostQuitMessage(0);
        } else {
            self.notice("Omarchy Overlay could not start a new copy of itself.");
        }
    }

    /// Stores the current look in the profile that was last applied from the menu.
    unsafe fn update_profile(&mut self) {
        let Some(name) = self.last_profile.clone() else {
            self.notice("Apply or save a profile from Setup → Profiles first. Built-in profiles cannot be changed.");
            return;
        };
        let mut c = self.config.clone();
        let Some(profile) = c
            .appearance_profiles
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(&name))
        else {
            self.notice(&format!("\"{name}\" is a built-in profile. Save a copy under Setup → Profiles to change it."));
            return;
        };
        *profile = crate::appearance_profiles::Profile::capture(&self.config, name.clone());
        if self.commit(c) {
            self.show_feedback(&format!("Profile \"{name}\" updated"));
        }
    }

    /// Opens a work program by role, bringing an open one forward first where that is usual.
    unsafe fn open_app(&mut self, role: &str) {
        let opened = match role {
            "terminal" => {
                self.action(2);
                true
            }
            "browser" | "browser-private" => {
                let (program, private) = browser();
                let switch = if role == "browser" {
                    "--new-window"
                } else {
                    private
                };
                let switch = if program == "firefox.exe" && role == "browser" {
                    "-new-window"
                } else {
                    switch
                };
                shell(Path::new(program), switch) || shell(Path::new("msedge.exe"), "")
            }
            // Plain explorer.exe opens whichever file manager Windows is set to use (Files on
            // the user's PC); the This PC folder id always opens File Explorer itself.
            "files" => shell(Path::new("explorer.exe"), ""),
            "explorer" => shell(
                Path::new("explorer.exe"),
                "shell:::{20D04FE0-3AEA-1069-A2D8-08002B30309D}",
            ),
            "email" => match window_of(&["OUTLOOK.EXE", "olk.exe"]) {
                Some(task) => tasks::activate(&task, false),
                None => {
                    shell(Path::new("outlook.exe"), "")
                        || shell(Path::new("olk.exe"), "")
                        || shell(Path::new("mailto:"), "")
                }
            },
            "new-email" => shell(Path::new("mailto:"), ""),
            "calendar" => {
                shell(Path::new("outlook.exe"), "/select outlook:calendar")
                    || shell(Path::new("outlookcal:"), "")
            }
            "teams" => match window_of(&["ms-teams.exe", "Teams.exe"]) {
                Some(task) => tasks::activate(&task, false),
                None => shell(Path::new("ms-teams.exe"), "") || shell(Path::new("msteams:"), ""),
            },
            "editor" => shell(Path::new("notepad.exe"), ""),
            "calculator" => shell(Path::new("calc.exe"), ""),
            "activity" => shell(Path::new("taskmgr.exe"), ""),
            "music" => shell(Path::new("spotify:"), "") || shell(Path::new("mswindowsmusic:"), ""),
            _ => true,
        };
        if !opened {
            let label = crate::commands::APPS
                .iter()
                .find(|(id, _)| *id == role)
                .map_or(role, |(_, label)| label);
            self.notice(&format!("{label} could not be opened on this PC."));
        }
    }

    /// An open window first, then the Chrome Apps shortcut, then an offer to open the site.
    unsafe fn open_webapp(&mut self, name: &str, confirmed: bool) {
        let Some(app) = self
            .webapps
            .iter()
            .find(|w| w.name.eq_ignore_ascii_case(name))
            .cloned()
        else {
            self.show_feedback(&format!("webapps.json has no app called \"{name}\"."));
            return;
        };
        if confirmed {
            if !shell(Path::new(&app.url), "") {
                self.notice(&format!("{} could not be opened.", app.url));
            }
            return;
        }
        if let Some(task) = tasks::collect()
            .into_iter()
            .find(|t| app.matches(&t.identity.label, &t.title))
        {
            tasks::activate(&task, false);
            return;
        }
        if self.entries.is_empty() || self.command_menu {
            self.command_menu = false;
            self.refresh_apps();
        }
        // 0.19: the picker names a web app "<name> (web app)" when a program has its name.
        let marked = format!("{} (web app)", app.shortcut());
        if let Some(index) = [marked.as_str(), app.shortcut()].iter().find_map(|name| {
            self.entries
                .iter()
                .position(|e| e.action.is_none() && e.label.eq_ignore_ascii_case(name))
        }) {
            self.launch_entry(index);
            return;
        }
        self.open_level(
            Level::Confirm(
                Box::new(Command::WebApp(app.name.clone())),
                format!(
                    "{} is not installed as an app. Open {} in the browser? Setup → Install → Web app shows how to install it.",
                    app.name, app.url
                ),
            ),
            true,
        );
    }

    /// Runs a program or script that sits inside the config folder.
    unsafe fn launch_from_config(&mut self, program: &str, arguments: &str) {
        let folder = config_folder::folder();
        let relative = Path::new(program);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            self.notice(&format!(
                "launch {program}: only files inside the config folder can be started."
            ));
            return;
        }
        let path = folder.join(relative);
        if !path.is_file() {
            self.notice(&format!("launch: {} does not exist.", path.display()));
            return;
        }
        if !shell_in(&path, arguments, Some(&folder)) {
            self.notice(&format!("launch: {} could not be started.", path.display()));
        }
    }

    /// Re-registers every Omarchy Overlay key for the current keymap. Paused, nothing is held.
    pub(super) unsafe fn restart_keys(&mut self) {
        keyhook::stop();
        // A stopped hook never sends the button-up, so end any Super + drag here.
        self.mouse_drag_event(keyhook::DRAG_END);
        for id in self.hotkeys.drain(..) {
            UnregisterHotKey(self.main, id);
        }
        self.cycle_binding = None;
        self.failures.clear();
        if !self.keys_paused {
            if self.config.keymap != "omarchy" {
                self.register_hotkeys();
                self.sync_cycle_hotkey();
            }
            if self.config.keymap != "classic" {
                match keyhook::start(self.main, &self.binds, &self.config.tap_super) {
                    Ok(generation) => self.bind_generation = generation,
                    Err(error) => {
                        log(&error);
                        self.failures.push(error);
                    }
                }
            }
        }
        keyhook::TILING.store(self.active, std::sync::atomic::Ordering::Relaxed);
        self.update_help();
    }
}
