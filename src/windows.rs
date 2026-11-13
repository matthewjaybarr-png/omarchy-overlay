use crate::layout::{self, Rect};
use crate::rules::{self, AppRule, Mode};
use crate::{config::Config, profile};
#[path = "app_rules_ui.rs"]
mod app_rules_ui;
#[path = "appearance_effects.rs"]
mod appearance_effects;
#[path = "appearance_options.rs"]
mod appearance_options;
#[path = "appearance_ui.rs"]
mod appearance_ui;
#[path = "applications_ui.rs"]
mod applications_ui;
#[path = "bar.rs"]
mod bar;
#[path = "bar_editor.rs"]
mod bar_editor;
#[path = "bar_icons.rs"]
mod bar_icons;
#[path = "bar_settings.rs"]
mod bar_settings;
#[path = "command_menu.rs"]
mod command_menu;
#[path = "command_run.rs"]
mod command_run;
#[path = "config_folder.rs"]
mod config_folder;
#[path = "desktop_actions.rs"]
mod desktop_actions;
#[path = "focus_outline.rs"]
mod focus_outline;
#[path = "fonts.rs"]
mod fonts;
#[path = "keyhook.rs"]
mod keyhook;
#[path = "module_settings.rs"]
mod module_settings;
#[path = "native_status.rs"]
mod native_status;
#[path = "omarchy_keys.rs"]
mod omarchy_keys;
#[path = "opacity.rs"]
mod opacity;
#[path = "osd.rs"]
mod osd;
#[path = "overlay.rs"]
mod overlay;
#[path = "overlay_pages.rs"]
mod overlay_pages;
#[path = "panel_scroll.rs"]
mod panel_scroll;
#[path = "picker_ui.rs"]
mod picker_ui;
#[path = "profiles_actions.rs"]
mod profiles_actions;
#[path = "profiles_ui.rs"]
mod profiles_ui;
#[path = "recovery.rs"]
mod recovery;
#[path = "resize.rs"]
mod resize;
#[path = "scrolling_runtime.rs"]
mod scrolling_runtime;
#[path = "settings.rs"]
mod settings;
#[path = "shortcuts_ui.rs"]
mod shortcuts_ui;
#[path = "taskbar_visibility.rs"]
mod taskbar_visibility;
#[path = "tasks.rs"]
mod tasks;
#[path = "theme.rs"]
mod theme;
#[path = "tray_popup.rs"]
mod tray_popup;
#[path = "ui_choices.rs"]
mod ui_choices;
#[path = "window_actions.rs"]
mod window_actions;
#[path = "window_motion.rs"]
mod window_motion;
#[path = "window_theme.rs"]
mod window_theme;
#[path = "windows_accent.rs"]
mod windows_accent;
#[path = "windows_personalization.rs"]
mod windows_personalization;
#[path = "workspace_ui.rs"]
mod workspace_ui;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ffi::c_void,
    fs,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    Security::*,
    System::{LibraryLoader::*, Threading::*},
    UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
};

const NAME: &str = "Omarchy Overlay";
const TRAY_MESSAGE: u32 = WM_APP + 1;
const MAIN_CLASS: &str = "OmarchyOverlay.Main";
const LAUNCH_CLASS: &str = "OmarchyOverlay.Launcher";
const APPEARANCE_CLASS: &str = "OmarchyOverlay.Appearance";
const HELP_CLASS: &str = "OmarchyOverlay.Help";
const BAR_ACTION: u32 = WM_APP + 10;
const BAR_SCROLL: u32 = WM_APP + 58;
const OSD_MESSAGE: u32 = WM_APP + 40;
const BASE: u32 = 100;
const CYCLE_HOTKEY: i32 = BASE as i32 + 38;
// The Windows headers define TBM_GETPOS as WM_USER; metadata omits that alias.
const TBM_GETPOS: u32 = WM_USER;
const SS_PATHELLIPSIS: u32 = 0x8000;
/// Why Omarchy Overlay is exiting: 0 normal, 1 "Exit before removing the stick", 2 the drive
/// holding OmarchyOverlay.exe went away (0.18 note item 5), 3 the drive is still there but
/// OmarchyOverlay.exe is gone (deleted, moved, or removed by security software; 0.19).
static EXIT_REASON: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// Consecutive checks that found OmarchyOverlay.exe missing.
static EXE_MISSING: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// OmarchyOverlay.exe runs from a removable drive (set once at startup). Off on a fixed
/// drive such as a copy in the user's own folder.
static ON_STICK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const STICK_TIMER: usize = 5;
/// Restores the desktop and exits, then says the stick can be removed.
unsafe fn exit_for_stick() {
    EXIT_REASON.store(1, std::sync::atomic::Ordering::Relaxed);
    PostQuitMessage(0);
}
/// Runs every two seconds. Windows keeps the program in memory (/SWAPRUN), so when the
/// stick is pulled out Omarchy Overlay notices its file is gone and puts the desktop back
/// instead of leaving the taskbar hidden and windows tiled.
unsafe fn check_stick() {
    use std::sync::atomic::Ordering::Relaxed;
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    if exe.exists() {
        EXE_MISSING.store(0, Relaxed);
    } else if EXE_MISSING.fetch_add(1, Relaxed) + 1 >= 2 && EXIT_REASON.load(Relaxed) == 0 {
        if exe.ancestors().last().is_some_and(|root| root.exists()) {
            log(&format!("OmarchyOverlay.exe is gone from {} but its drive is still there (deleted, moved, or removed by security software); restoring the desktop and exiting.", exe.display()));
            EXIT_REASON.store(3, Relaxed);
        } else {
            log("The drive holding OmarchyOverlay.exe was removed; restoring the desktop and exiting.");
            EXIT_REASON.store(2, Relaxed);
        }
        PostQuitMessage(0);
    }
}

#[derive(Clone)]
struct Entry {
    label: String,
    path: PathBuf,
    arguments: String,
    custom_key: Option<String>,
    working_directory: Option<PathBuf>,
    action: Option<command_menu::Action>,
}
impl Entry {
    fn available(&self) -> bool {
        if self.action.is_some() {
            return true;
        }
        self.custom_key.is_none()
            || (self.path.exists() && self.working_directory.as_ref().is_none_or(|p| p.is_dir()))
    }
    fn key(&self) -> String {
        if let Some(action) = &self.action {
            return action.key();
        }
        if let Some(key) = &self.custom_key {
            return key.clone();
        }
        format!(
            "{}\n{}",
            self.path.to_string_lossy().to_lowercase(),
            self.arguments
        )
    }
}
struct Saved {
    placement: WINDOWPLACEMENT,
    pid: u32,
    normal_screen: RECT,
    marker: usize,
}
const PLACEMENT_OWNER: &str = "OmarchyOverlay.PlacementOwner.v1";
impl Saved {
    unsafe fn matches(&self, hwnd: HWND) -> bool {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        IsWindow(hwnd) != 0
            && pid == self.pid
            && GetPropW(hwnd, wide(PLACEMENT_OWNER).as_ptr()) as usize == self.marker
    }
    unsafe fn restore(&self, hwnd: HWND) -> bool {
        if !self.matches(hwnd) {
            return true;
        }
        if !responsive(hwnd) {
            return false;
        }
        // Restore an engine-minimized window before applying its original normal
        // rectangle. Some applications reapply their cached tiled bounds during
        // the minimize/restore transition and would otherwise overwrite it.
        if scrolling_runtime::owned(hwnd) && IsIconic(hwnd) != 0 {
            ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
            for _ in 0..30 {
                if IsIconic(hwnd) == 0 {
                    break;
                }
                Sleep(10);
            }
            if IsIconic(hwnd) != 0 {
                return false;
            }
        }
        if !self.matches(hwnd) {
            return true;
        }
        // WINDOWPLACEMENT uses workspace coordinates. A top appbar changes that
        // origin; retain an absolute normal rectangle and rebase it at restore.
        let monitor = MonitorFromRect(&self.normal_screen, MONITOR_DEFAULTTONEAREST);
        let (dx, dy) = workspace_offset(monitor);
        let mut placement = self.placement;
        placement.rcNormalPosition = self.normal_screen;
        OffsetRect(&mut placement.rcNormalPosition, -dx, -dy);
        // A user-minimized window must stay minimized while its normal rectangle is restored.
        if IsIconic(hwnd) != 0 && !scrolling_runtime::owned(hwnd) {
            placement.showCmd = SW_SHOWMINNOACTIVE as u32;
        }
        if SetWindowPlacement(hwnd, &placement) == 0 {
            // A busy window can refuse once; retry before reporting it by name.
            Sleep(50);
            if SetWindowPlacement(hwnd, &placement) == 0 {
                log(&format!(
                    "Could not restore the original placement of \"{}\" ({}).",
                    text_of(hwnd),
                    process_name(hwnd)
                ));
                return false;
            }
        }
        RemovePropW(hwnd, wide(scrolling_runtime::HIDDEN).as_ptr());
        RemovePropW(hwnd, wide(PLACEMENT_OWNER).as_ptr());
        true
    }
}
unsafe fn workspace_offset(monitor: HMONITOR) -> (i32, i32) {
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return (0, 0);
    }
    (
        info.rcWork.left - info.rcMonitor.left,
        info.rcWork.top - info.rcMonitor.top,
    )
}
type LayoutSession = (
    HashMap<isize, resize::TiledGroup>,
    HashMap<isize, crate::dwindle::Tree>,
    Vec<isize>,
);
struct App {
    main: HWND,
    launcher: HWND,
    edit: HWND,
    list: HWND,
    status: HWND,
    toggle: HWND,
    tray: NOTIFYICONDATAW,
    font: HFONT,
    brush: HBRUSH,
    appearance: HWND,
    appearance_profiles: HWND,
    appearance_options: HWND,
    bar_pick: (usize, usize),
    rule_center: HWND,
    rule_windows: Vec<tasks::Task>,
    appearance_preview: Option<crate::appearance_profiles::Profile>,
    outline: HWND,
    osd: HWND,
    taskbar_hidden_preferred: bool,
    pending_system_action: Option<(u32, std::time::Instant)>,
    selected_label: HWND,
    opacity_label: HWND,
    bar_label: HWND,
    inactive_label: HWND,
    selected: HWND,
    opacity: opacity::Engine,
    frames: window_theme::Engine,
    settings: settings::Panel,
    routed: HashMap<isize, Route>,
    moves: HashMap<isize, Route>,
    config: Config,
    directory: PathBuf,
    profile: profile::Active,
    profiles: HWND,
    bar_settings: HWND,
    module_settings: HWND,
    tasks: tasks::Panel,
    help: HWND,
    custom_app: Option<usize>,
    active: bool,
    motions: HashMap<isize, window_motion::Motion>,
    tiled: HashMap<isize, resize::TiledGroup>,
    dwindle: HashMap<isize, crate::dwindle::Tree>,
    scrolling: HashMap<(String, isize), crate::scrolling::State>,
    last_tiled_focus: Option<isize>,
    layout_preferences: crate::workspace_layouts::Preferences,
    layout_all_desktops: bool,
    layout_monitor: String,
    layout_desktop: String,
    scratchpads: HashMap<String, window_actions::Scratchpad>,
    menu_target: Option<tasks::Task>,
    layout_sessions: HashMap<String, LayoutSession>,
    drag: Option<resize::Drag>,
    order: Vec<isize>,
    floating: HashSet<isize>,
    saved: HashMap<isize, Saved>,
    signature: Vec<(isize, isize, Rect)>,
    dirty: bool,
    entries: Vec<Entry>,
    filtered: Vec<usize>,
    command_menu: bool,
    picker_history: crate::picker::History,
    picker_icons: HashMap<String, std::rc::Rc<picker_ui::Icon>>,
    hotkeys: Vec<i32>,
    cycle_binding: Option<crate::shortcuts::Binding>,
    cycle_error: Option<String>,
    failures: Vec<String>,
    /// Super keys from config\binds.conf (or the built-in list).
    binds: Vec<crate::binds::Bind>,
    menus: crate::menu_model::Menus,
    webapps: Vec<crate::webapps::WebApp>,
    /// One line per config file that was skipped, naming the file and line.
    config_errors: Vec<String>,
    overlay_undo: Option<overlay_pages::Undo>,
    /// A destructive menu row waiting for its second Enter.
    confirm_choice: Option<(&'static str, String)>,
    /// True while every key is given back to Windows.
    keys_paused: bool,
    keep_awake: bool,
    pending_command: Option<(crate::commands::Command, bool)>,
    /// Sent with each Super key message so a key pressed before a reload is ignored.
    bind_generation: usize,
    /// The saved profile last applied or saved from the menu.
    last_profile: Option<String>,
}
struct Route {
    pid: u32,
    device: String,
    pending: u8,
}
unsafe fn window_class(hwnd: HWND) -> String {
    let mut text = [0u16; 256];
    let count = GetClassNameW(hwnd, text.as_mut_ptr(), 256);
    String::from_utf16_lossy(&text[..count.max(0) as usize])
}
unsafe fn rule_for(config: &Config, hwnd: HWND) -> Option<&AppRule> {
    rules::matching(
        &config.rules,
        &process_name(hwnd),
        &text_of(hwnd),
        &window_class(hwnd),
    )
}
unsafe fn window_policy(config: &Config, hwnd: HWND) -> rules::Policy {
    config.policy(&process_name(hwnd), &text_of(hwnd), &window_class(hwnd))
}
unsafe fn can_change_opacity(config: &Config, hwnd: HWND) -> bool {
    !config
        .excluded_processes
        .iter()
        .any(|name| process_name(hwnd).eq_ignore_ascii_case(name))
        && window_policy(config, hwnd).mode != Mode::Ignore
        && window_policy(config, hwnd).allow_opacity
}
thread_local! { static APP: RefCell<Option<App>> = const { RefCell::new(None) }; }

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
/// Asks the running copy to restore the desktop and exit; true once it has (or none ran).
unsafe fn close_running() -> bool {
    let other = FindWindowW(wide(MAIN_CLASS).as_ptr(), null());
    if other.is_null() {
        return true;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(other, &mut pid);
    let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
    PostMessageW(other, WM_HOTKEY, (BASE + 5) as usize, 0);
    let closed = !process.is_null() && WaitForSingleObject(process, 10000) == WAIT_OBJECT_0;
    if !process.is_null() {
        CloseHandle(process);
    }
    closed
}

/// `--install [--user DOMAIN\name] [--yes] [--quiet]` and
/// `--uninstall [--remove-settings] [--quiet]` (Admin edition).
/// `--quiet` shows no message boxes (and answers yes); the exit code says how it went:
/// 0 done, 1 failed, 2 cancelled or the administrator prompt was refused.
unsafe fn install_command(arguments: &[String], directory: &Path) -> i32 {
    let installing = arguments[0] == "--install";
    let has = |flag: &str| arguments.iter().any(|a| a == flag);
    let quiet = has("--quiet");
    let remove_settings = has("--remove-settings");
    let say = |text: &str| {
        log(&format!("Installer: {}", text.replace("\r\n", " ")));
        if !quiet {
            message(text);
        }
    };
    let user = arguments
        .iter()
        .position(|a| a == "--user")
        .and_then(|i| arguments.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            let get = |name| std::env::var(name).unwrap_or_default();
            format!("{}\\{}", get("USERDOMAIN"), get("USERNAME"))
        });
    let this = env!("CARGO_PKG_VERSION");
    let plan = crate::install::current_plan();
    if installing && !quiet && !has("--yes") {
        // Asked before the administrator prompt, so saying no changes nothing.
        if let Some(question) = crate::install_files::question(&plan, this) {
            let answer = MessageBoxW(
                null_mut(),
                wide(&question).as_ptr(),
                wide(NAME).as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            );
            if answer != IDYES {
                return 2;
            }
        }
    }
    if !crate::launch_unelevated::process_elevated() {
        // The task must run as the signed-in user, so pass that name to the elevated copy.
        let mut relaunch = if installing {
            format!("--install --yes --user {}", crate::launch_unelevated::quote(&user))
        } else if remove_settings {
            "--uninstall --remove-settings".into()
        } else {
            "--uninstall".into()
        };
        if quiet {
            relaunch.push_str(" --quiet");
        }
        return match crate::install::elevate(&relaunch) {
            Some(code) => code as i32,
            None => {
                if !quiet {
                    message("Omarchy Overlay needs administrator rights to install or uninstall. Nothing was changed.");
                }
                2
            }
        };
    }
    // One installer at a time, across every signed-in user.
    let installer = CreateMutexW(null(), 1, wide("Global\\OmarchyOverlay.Installer").as_ptr());
    if installer.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
        if !installer.is_null() {
            CloseHandle(installer);
        }
        say("Another Omarchy Overlay install or uninstall is running. Try again when it has finished.");
        return 1;
    }
    let result = install_locked(installing, &plan, &user, remove_settings, directory, &say);
    ReleaseMutex(installer);
    CloseHandle(installer);
    result
}

unsafe fn install_locked(
    installing: bool,
    plan: &crate::install_files::Plan,
    user: &str,
    remove_settings: bool,
    directory: &Path,
    say: &dyn Fn(&str),
) -> i32 {
    log(&format!(
        "Installer: {} {} ({plan:?}) for {user}",
        if installing { "install" } else { "uninstall" },
        env!("CARGO_PKG_VERSION")
    ));
    if !close_running() {
        say("Omarchy Overlay did not exit. Close it and try again. Nothing was changed.");
        return 1;
    }
    let lock = match recovery::Lock::acquire() {
        Ok(lock) => lock,
        Err(error) => {
            say(&error);
            return 1;
        }
    };
    // The started copy takes this lock at startup, so release it before starting the task or
    // waiting on a message box; held, the new copy gives up after 10 seconds.
    if installing {
        let installed = crate::install::install(user);
        drop(lock);
        return match installed {
            Ok(report) => {
                if let Err(error) = crate::install::start_task() {
                    log(&format!("Installer: could not start the logon task: {error}"));
                }
                say(&crate::install_files::success_message(
                    plan,
                    env!("CARGO_PKG_VERSION"),
                    &crate::install::install_dir(),
                    user,
                    !report.pending_delete.is_empty(),
                ));
                0
            }
            Err(error) => {
                say(&format!(
                    "Install failed: {error}\r\n\r\nThe installed copy, if there was one, was left as it was. If another signed-in user is running Omarchy Overlay, close it there and try again."
                ));
                1
            }
        };
    }
    let mut errors = recovery::recover(directory).errors;
    errors.extend(crate::install::remove(remove_settings.then_some(directory)));
    drop(lock);
    // The installed copy's recovery guard can outlive it by a moment.
    crate::install::wait_for_programs_in(&crate::install::install_dir(), 10_000);
    say(&if errors.is_empty() {
        "Omarchy Overlay is uninstalled and the desktop is restored.".to_string()
    } else {
        format!("Uninstall finished with problems:\r\n{}", errors.join("\r\n"))
    });
    crate::install::remove_folder_after_exit();
    i32::from(!errors.is_empty())
}

fn data_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OmarchyOverlay")
}
unsafe fn responsive(hwnd: HWND) -> bool {
    let mut result = 0;
    SendMessageTimeoutW(
        hwnd,
        WM_NULL,
        0,
        0,
        SMTO_ABORTIFHUNG | SMTO_BLOCK,
        200,
        &mut result,
    ) != 0
}
fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    profile::write_json(path, value)
}
pub(crate) fn log(message: &str) {
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_directory().join("omarchy-overlay.log"))
    {
        let _ = writeln!(f, "{message}");
    }
}
unsafe fn message(text: &str) {
    MessageBoxW(
        null_mut(),
        wide(text).as_ptr(),
        wide(NAME).as_ptr(),
        MB_OK | MB_ICONINFORMATION,
    );
}
unsafe fn set_text(hwnd: HWND, text: &str) {
    SetWindowTextW(hwnd, wide(text).as_ptr());
    // Transparent material labels must erase the old text through their parent.
    let parent = GetParent(hwnd);
    if !parent.is_null() && window_theme::backdrop_active(parent) {
        RedrawWindow(
            parent,
            null(),
            null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN,
        );
    }
}
unsafe fn text_of(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; (GetWindowTextLengthW(hwnd) + 1).max(1) as usize];
    let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
    String::from_utf16_lossy(&buffer[..len as usize])
}
// This helper mirrors CreateWindowEx's control parameters.
#[allow(clippy::too_many_arguments)]
unsafe fn child(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> HWND {
    let control = CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        parent,
        id as _,
        GetModuleHandleW(null()),
        null(),
    );
    if !control.is_null() {
        let dark = theme::current().dark;
        theme_control(control, dark);
        if dark {
            // The window's own scroll bars follow its theme.
            windows_sys::Win32::UI::Controls::SetWindowTheme(
                parent,
                wide("DarkMode_Explorer").as_ptr(),
                null(),
            );
        }
    }
    control
}
/// 0.21: the manifest now asks for Common Controls 6, so controls use visual styles.
/// Themed controls ignore part of WM_CTLCOLOR*, so on a dark palette each class gets
/// Windows' own dark style: the file dialogs' style for dropdowns and text boxes,
/// Explorer's for lists and scroll bars. Check boxes and radio buttons draw their
/// label in the theme's black, so on dark they stay unthemed (as before 0.21) and
/// keep Omarchy Overlay's text colour. On a light palette every control uses the normal theme.
unsafe fn theme_control(control: HWND, dark: bool) {
    let style = crate::control_theme::dark_style(
        &class_of(control),
        GetWindowLongPtrW(control, GWL_STYLE) as u32,
        dark,
    );
    let (name, list) = match style {
        Some("") => (wide(""), wide("")),
        Some(name) => (wide(name), Vec::new()),
        None => (Vec::new(), Vec::new()),
    };
    windows_sys::Win32::UI::Controls::SetWindowTheme(
        control,
        if name.is_empty() {
            null()
        } else {
            name.as_ptr()
        },
        if list.is_empty() {
            null()
        } else {
            list.as_ptr()
        },
    );
}
unsafe fn class_of(hwnd: HWND) -> String {
    let mut class = [0u16; 64];
    let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32).max(0) as usize;
    String::from_utf16_lossy(&class[..len])
}
/// Re-applies the control styles after a theme change, and gives the window's own
/// scroll bars Explorer's dark style.
unsafe fn theme_children(hwnd: HWND, dark: bool) {
    unsafe extern "system" fn each(control: HWND, dark: LPARAM) -> i32 {
        theme_control(control, dark != 0);
        1
    }
    if hwnd.is_null() {
        return;
    }
    let name = wide(if dark {
        "DarkMode_Explorer"
    } else {
        "Explorer"
    });
    windows_sys::Win32::UI::Controls::SetWindowTheme(hwnd, name.as_ptr(), null());
    EnumChildWindows(hwnd, Some(each), isize::from(dark));
}
unsafe fn shell(path: &Path, arguments: &str) -> bool {
    shell_in(path, arguments, None)
}
unsafe fn shell_in(path: &Path, arguments: &str, directory: Option<&Path>) -> bool {
    if crate::launch_unelevated::process_elevated() {
        // Admin mode never starts apps elevated; with no helper the launch fails.
        let started = crate::launch_unelevated::shell_unelevated(path, arguments, directory);
        if !started {
            log(&format!("Could not start {} without elevation.", path.display()));
        }
        return started;
    }
    shell_direct(path, arguments, directory)
}
unsafe fn shell_direct(path: &Path, arguments: &str, directory: Option<&Path>) -> bool {
    let directory = directory.map(|p| wide(&p.to_string_lossy()));
    let arguments_wide = wide(arguments);
    let result = ShellExecuteW(
        null_mut(),
        wide("open").as_ptr(),
        wide(&path.to_string_lossy()).as_ptr(),
        if arguments.is_empty() {
            null()
        } else {
            arguments_wide.as_ptr()
        },
        directory.as_ref().map_or(null(), |p| p.as_ptr()),
        SW_SHOWNORMAL,
    );
    result as isize > 32
}
unsafe fn register_class(name: &str, brush: HBRUSH) {
    let class_name = wide(name);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: GetModuleHandleW(null()),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hIcon: app_icon(),
        hbrBackground: brush,
        lpszClassName: class_name.as_ptr(),
        ..zeroed()
    };
    // The class name must outlive the call, not the registration.
    RegisterClassW(&class);
}

unsafe fn app_icon() -> HICON {
    // Shared resource handles live with the module; the OS releases them on exit.
    LoadIconW(GetModuleHandleW(null()), 101usize as *const u16)
}

unsafe fn about(owner: HWND) {
    let settings = match profile::settings() {
        Some(folder) => format!(
            "Settings folder (shared by every version):\r\n{}",
            folder.display()
        ),
        None => format!("Settings and config files:\r\n{}", data_directory().display()),
    };
    let program = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let installed = std::env::current_exe().is_ok_and(|exe| crate::install::is_installed(&exe));
    let tail = if installed {
        "Installed for this PC; starts when you sign in. Remove it from Settings > Apps."
    } else if ON_STICK.load(std::sync::atomic::Ordering::Relaxed) {
        "Runs from its folder on the stick; nothing is installed.\r\nUse \"Exit before removing the stick\" before pulling the drive."
    } else {
        "Runs from its folder; nothing is installed."
    };
    let text =
        wide(&format!(
            concat!("Omarchy Overlay ", env!("CARGO_PKG_VERSION"),
        "\r\nKeyboard-first tiling window manager for Windows 11.\r\n\r\nProgram: {}\r\n{}\r\n\r\n{}"),
            program, settings, tail
        ));
    let caption = wide("About Omarchy Overlay");
    let parameters = MSGBOXPARAMSW {
        cbSize: size_of::<MSGBOXPARAMSW>() as u32,
        hwndOwner: owner,
        hInstance: GetModuleHandleW(null()),
        lpszText: text.as_ptr(),
        lpszCaption: caption.as_ptr(),
        dwStyle: MB_OK | MB_USERICON,
        lpszIcon: 101usize as *const u16,
        ..zeroed()
    };
    MessageBoxIndirectW(&parameters);
}

pub fn run() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let directory = data_directory();
        if let Err(error) = fs::create_dir_all(&directory) {
            message(&format!("Cannot create settings folder: {error}"));
            return;
        }
        let mut arguments: Vec<String> = std::env::args().skip(1).collect();
        if arguments.first().is_some_and(|v| v == "--restart") {
            // Restart from the menu: wait for the old copy to restore the desktop and exit.
            if let Some(pid) = arguments.get(1).and_then(|v| v.parse::<u32>().ok()) {
                let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                if !process.is_null() {
                    WaitForSingleObject(process, 10000);
                    CloseHandle(process);
                }
            }
            arguments.drain(..arguments.len().min(2));
        }
        if let Some((path, args, folder)) = crate::launch_unelevated::parse_helper_args(&arguments) {
            // Started by an elevated instance through Explorer: open the item and exit.
            std::process::exit(if shell_direct(&path, &args, folder.as_deref()) { 0 } else { 1 });
        }
        if matches!(arguments.first().map(String::as_str), Some("--install" | "--uninstall")) {
            std::process::exit(install_command(&arguments, &directory));
        }
        if let [flag, path] = arguments.as_slice() {
            if flag == "--check-profile" {
                let result =
                    profile::read_text(Path::new(path)).and_then(|text| profile::parse(&text));
                let (valid, error) = match result {
                    Ok(_) => (true, None),
                    Err(error) => (false, Some(error)),
                };
                println!("{}", serde_json::json!({"valid":valid,"error":error}));
                std::process::exit(if valid { 0 } else { 2 });
            }
        }
        if arguments.first().is_some_and(|v| v == "--watchdog") {
            if let (Some(pid), Some(created)) = (
                arguments.get(1).and_then(|v| v.parse().ok()),
                arguments.get(2).and_then(|v| v.parse().ok()),
            ) {
                recovery::watchdog(pid, created, &directory);
            }
            return;
        }
        let explicit = match arguments.as_slice() {
            [] => None,
            [flag, path] if flag == "--profile" => Some(PathBuf::from(path)),
            [flag] if flag == "--recover" => None,
            _ => {
                message("Usage: OmarchyOverlay.exe [--profile path-to-profile.json] or OmarchyOverlay.exe --recover");
                return;
            }
        };
        let startup_lock = match recovery::Lock::acquire() {
            Ok(lock) => lock,
            Err(error) => {
                message(&error);
                return;
            }
        };
        let mut mutex = CreateMutexW(null(), 0, wide(recovery::INSTANCE).as_ptr());
        if mutex.is_null() {
            message("Could not create the session lock.");
            return;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let other = FindWindowW(wide(MAIN_CLASS).as_ptr(), null());
            CloseHandle(mutex);
            if arguments.first().is_some_and(|v| v == "--recover") {
                let mut pid = 0;
                GetWindowThreadProcessId(other, &mut pid);
                let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                PostMessageW(other, WM_HOTKEY, (BASE + 5) as usize, 0);
                if process.is_null() || WaitForSingleObject(process, 3000) != WAIT_OBJECT_0 {
                    if !process.is_null() {
                        CloseHandle(process);
                    }
                    message("Omarchy Overlay did not exit. Close its main process and run recovery again; other applications will not be stopped.");
                    return;
                }
                CloseHandle(process);
                mutex = CreateMutexW(null(), 0, wide(recovery::INSTANCE).as_ptr());
                if mutex.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
                    if !mutex.is_null() {
                        CloseHandle(mutex);
                    }
                    message("Another Omarchy Overlay instance is starting. Try recovery again.");
                    return;
                }
            } else {
                // Starting Omarchy Overlay again opens the menu of the copy that is running.
                let mut pid = 0;
                GetWindowThreadProcessId(other, &mut pid);
                AllowSetForegroundWindow(pid);
                PostMessageW(other, BAR_ACTION, 2, 0);
                return;
            }
        }
        let report = recovery::recover(&directory);
        if !report.errors.is_empty() {
            message(&format!(
                "Recovery needs attention. Saved records were preserved.\r\n{}",
                report.errors.join("\r\n")
            ));
            CloseHandle(mutex);
            return;
        }
        if arguments.first().is_some_and(|v| v == "--recover") {
            message("Desktop recovery completed. Omarchy Overlay has not changed your profile.");
            CloseHandle(mutex);
            return;
        }
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                message(&error.to_string());
                CloseHandle(mutex);
                return;
            }
        };
        let stick = executable.ancestors().last().is_some_and(|root| {
            // DRIVE_REMOVABLE
            windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(
                wide(&root.to_string_lossy()).as_ptr(),
            ) == 2
        });
        ON_STICK.store(stick, std::sync::atomic::Ordering::Relaxed);
        let mut active_profile = match profile::find_for(&executable, &directory, explicit, crate::install::is_installed(&executable)) {
            profile::Found::Use(active) => active,
            profile::Found::Missing { folder, source } => {
                let (text, buttons) = match &source {
                    Some(source) => (format!(
                        "Omarchy Overlay keeps your settings in one folder that every version uses:\r\n{}\r\n\r\nThat folder does not exist yet. Your newest profile was found here:\r\n{}\r\n\r\nYes: create the folder and copy these settings, their config files and wallpapers into it (the original is not changed).\r\nNo: create the folder with default settings.\r\nCancel: exit without changing anything.",
                        folder.display(), profile::describe(source)), MB_YESNOCANCEL),
                    None => (format!(
                        "Omarchy Overlay keeps your settings in one folder that every version uses:\r\n{}\r\n\r\nThat folder does not exist and no earlier profile was found.\r\n\r\nOK: create the folder with default settings.\r\nCancel: exit without changing anything.",
                        folder.display()), MB_OKCANCEL),
                };
                let answer = MessageBoxW(
                    null_mut(),
                    wide(&text).as_ptr(),
                    wide("Omarchy Overlay settings folder is missing").as_ptr(),
                    buttons | MB_ICONWARNING | MB_SETFOREGROUND,
                );
                let copy = match answer {
                    IDYES => source.as_deref(),
                    IDNO | IDOK => None,
                    _ => {
                        CloseHandle(mutex);
                        return;
                    }
                };
                match profile::create_settings(&folder, copy) {
                    Ok(path) => {
                        log(&format!(
                            "Created settings folder {} from {}",
                            folder.display(),
                            copy.map_or("defaults".into(), |p| p.display().to_string())
                        ));
                        profile::Active {
                            path,
                            id: Some(String::new()),
                        }
                    }
                    Err(error) => {
                        message(&format!("Could not create the settings folder:\r\n{error}"));
                        CloseHandle(mutex);
                        return;
                    }
                }
            }
        };
        profile::remember_settings(&active_profile);
        log(&format!(
            "Omarchy Overlay {} from {} using profile {}",
            env!("CARGO_PKG_VERSION"),
            executable.display(),
            active_profile.path.display()
        ));
        let mut config = match profile::load(&mut active_profile, &directory) {
            Ok(config) => config,
            Err(error) => {
                message(&format!("{}\r\n{}", error, active_profile.path.display()));
                CloseHandle(mutex);
                return;
            }
        };
        if let Err(error) = profile::backup_before_upgrade(&active_profile, &config) {
            message(&format!(
                "Could not preserve the previous profile for rollback: {error}"
            ));
            CloseHandle(mutex);
            return;
        }
        let layout_preferences = (|| -> Result<crate::workspace_layouts::Preferences, String> {
            let path = directory.join("workspace-layouts.json");
            if !path.exists() {
                return Ok(Default::default());
            }
            let data: crate::workspace_layouts::Preferences =
                serde_json::from_str(&profile::read_text(&path)?).map_err(|e| e.to_string())?;
            data.validate()?;
            Ok(data)
        })();
        let layout_preferences = match layout_preferences {
            Ok(data) => data,
            Err(error) => {
                message(&format!("Could not load workspace layout choices: {error}"));
                CloseHandle(mutex);
                return;
            }
        };
        if let Err(error) = recovery::spawn_watchdog() {
            message(&error);
            CloseHandle(mutex);
            return;
        }
        drop(startup_lock);
        // Logs which font source it used: bundled, installed or none (K1).
        fonts::load();
        let config_files = config_folder::load();
        config.always_float = config_files.always_float;
        theme::set(&config.theme);
        let brush = CreateSolidBrush(theme::current().background);
        register_class(MAIN_CLASS, brush);
        register_class(LAUNCH_CLASS, brush);
        register_class(APPEARANCE_CLASS, brush);
        register_class(HELP_CLASS, brush);
        let main = CreateWindowExW(
            0,
            wide(MAIN_CLASS).as_ptr(),
            wide("Omarchy Overlay — desktop controls").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VSCROLL,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            780,
            770,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        let launcher = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            wide(LAUNCH_CLASS).as_ptr(),
            wide("Omarchy Overlay — find an app").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            740,
            580,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        if main.is_null() || launcher.is_null() {
            message("Could not create Omarchy Overlay's windows.");
            CloseHandle(mutex);
            return;
        }
        let appearance = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            wide(APPEARANCE_CLASS).as_ptr(),
            wide("Omarchy Overlay — appearance").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            660,
            540,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        let dark: i32 = i32::from(theme::current().dark);
        for hwnd in [main, launcher, appearance] {
            DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
        }
        let status = child(
            main,
            "STATIC",
            "PAUSED — your windows stay where you put them",
            0,
            0,
            24,
            22,
            700,
            28,
        );
        for (id, label, x) in [
            (1, "Enable tiling", 24),
            (2, "Search apps and actions", 260),
            (4, "Restore desktop and exit", 496),
        ] {
            child(
                main,
                "BUTTON",
                label,
                BS_OWNERDRAW as u32 | WS_TABSTOP,
                id,
                x,
                58,
                220,
                40,
            );
        }
        let toggle = GetDlgItem(main, 1);
        for (heading, y) in [
            ("WINDOWS & TILING", 116),
            ("BAR & APPEARANCE", 342),
            ("APPS & SHORTCUTS", 492),
            ("PROFILES & RECOVERY", 614),
            ("WINDOWS CONTROLS", 706),
        ] {
            // SS_NOPREFIX: show "&" instead of underlining the next character.
            child(main, "STATIC", heading, 0x80, 0, 24, y, 700, 24);
        }
        child(main, "STATIC", "", SS_PATHELLIPSIS, 21, 24, 144, 692, 24);
        child(main, "STATIC", "", 0, 22, 24, 310, 460, 24);
        for (id, label, x, y) in [
            (12, "Open windows", 24, 174),
            (15, "Next layout", 260, 174),
            (29, "Layouts by workspace", 496, 174),
            (16, "Make main pane", 24, 218),
            (17, "Float selected window", 260, 218),
            (27, "Rebalance tiles", 496, 218),
            (18, "Previous monitor", 24, 262),
            (19, "Next monitor", 260, 262),
            (25, "Shrink main", 496, 262),
            (26, "Grow main", 612, 262),
            (14, "Bar and taskbar settings", 24, 370),
            (30, "Edit bar modules", 260, 370),
            (8, "Appearance and opacity", 496, 370),
            (31, "Appearance profiles", 24, 414),
            (32, "Materials and motion", 260, 414),
            (6, "Show / hide top bar", 496, 414),
            (3, "Tiling settings", 24, 520),
            (33, "App rules", 260, 520),
            (34, "Custom apps", 496, 520),
            (5, "Keyboard shortcuts", 24, 564),
            (35, "Profiles and recovery", 24, 642),
            (28, "About", 260, 642),
            (10, "File Explorer", 24, 734),
            (11, "Quick settings", 260, 734),
            (20, "Notifications / calendar", 496, 734),
            (13, "Windows Task View", 24, 778),
            (7, "Windows taskbar", 260, 778),
        ] {
            child(
                main,
                "BUTTON",
                label,
                BS_OWNERDRAW as u32 | WS_TABSTOP,
                id,
                x,
                y,
                if id == 25 || id == 26 { 104 } else { 220 },
                38,
            );
        }
        child(main, "STATIC", "", 0, 23, 24, 828, 700, 24);
        let help = shortcuts_ui::create();
        DwmSetWindowAttribute(help, 20, &dark as *const _ as _, size_of::<i32>() as u32);
        let edit = child(
            launcher,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            10,
            18,
            18,
            688,
            34,
        );
        SendMessageW(
            edit,
            EM_SETCUEBANNER,
            0,
            wide("Type an app name…").as_ptr() as isize,
        );
        let list = child(
            launcher,
            "LISTBOX",
            "",
            WS_BORDER
                | WS_VSCROLL
                | WS_TABSTOP
                | LBS_NOTIFY as u32
                | LBS_NOINTEGRALHEIGHT as u32
                | LBS_OWNERDRAWFIXED as u32
                | LBS_HASSTRINGS as u32,
            11,
            18,
            66,
            688,
            280,
        );
        SendMessageW(
            list,
            LB_SETITEMHEIGHT,
            0,
            (32 * GetDpiForWindow(launcher).max(96) / 96) as isize,
        );
        child(
            launcher,
            "STATIC",
            "",
            SS_PATHELLIPSIS,
            15,
            18,
            358,
            688,
            24,
        );
        for (id, label, x, width) in [
            (12, "Open app", 18, 160),
            (13, "Pin favorite", 190, 160),
            (14, "Refresh apps", 362, 160),
            (16, "Manage apps", 534, 172),
        ] {
            child(
                launcher,
                "BUTTON",
                label,
                BS_OWNERDRAW as u32 | WS_TABSTOP,
                id,
                x,
                402,
                width,
                38,
            );
        }
        child(launcher, "STATIC", "Search names or abbreviations · Favorites and recent apps appear first\r\nEnter / double-click to open · ↑ ↓ to select · Escape to close", 0, 0, 18, 460, 688, 44);
        let common = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&common);
        child(appearance, "STATIC", "Theme", 0, 0, 24, 20, 90, 24);
        child(
            appearance,
            "BUTTON",
            "Profiles…",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            38,
            448,
            16,
            160,
            32,
        );
        let combo = child(
            appearance,
            "COMBOBOX",
            "",
            WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            30,
            124,
            16,
            300,
            200,
        );
        for (i, (name, label)) in theme::NAMES.iter().enumerate() {
            SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            if *name == config.theme {
                SendMessageW(combo, CB_SETCURSEL, i, 0);
            }
        }
        let bar_label = child(appearance, "STATIC", "", 0, 0, 24, 64, 590, 24);
        let inactive_label = child(appearance, "STATIC", "", 0, 0, 24, 144, 590, 24);
        let selected_label = child(
            appearance,
            "STATIC",
            "Focus an app window to select it",
            SS_PATHELLIPSIS,
            0,
            24,
            226,
            590,
            24,
        );
        let opacity_label = child(
            appearance,
            "STATIC",
            "Selected window: original opacity",
            0,
            0,
            24,
            254,
            590,
            24,
        );
        for (id, y, minimum, value) in [
            (31, 92, 50, config.bar_opacity),
            (32, 172, 70, config.inactive_opacity),
            (33, 284, 50, 100),
        ] {
            let slider = child(
                appearance,
                "msctls_trackbar32",
                "",
                TBS_AUTOTICKS | WS_TABSTOP,
                id,
                24,
                y,
                580,
                36,
            );
            SendMessageW(slider, TBM_SETRANGE, 1, (minimum | (100 << 16)) as isize);
            SendMessageW(slider, TBM_SETPOS, 1, value as isize);
        }
        child(
            appearance,
            "BUTTON",
            "Restore selected",
            BS_PUSHBUTTON as u32 | WS_TABSTOP,
            35,
            24,
            340,
            180,
            34,
        );
        child(
            appearance,
            "BUTTON",
            "Restore all opacity",
            BS_PUSHBUTTON as u32 | WS_TABSTOP,
            36,
            216,
            340,
            210,
            34,
        );
        child(appearance, "STATIC", "Opacity fades the whole window, including text. 100% restores\r\nthe original appearance. Inactive fading is off at 100%.\r\nTheme and bar/inactive settings are saved automatically.\r\nManual window opacity lasts until restored or Omarchy Overlay exits.",
            0, 0, 24, 394, 590, 90);
        for parent in [main, appearance] {
            unsafe extern "system" fn owner_draw(hwnd: HWND, _: LPARAM) -> i32 {
                let mut class = [0u16; 32];
                GetClassNameW(hwnd, class.as_mut_ptr(), 32);
                if String::from_utf16_lossy(&class).starts_with("Button") {
                    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                    SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !0xf) | BS_OWNERDRAW as isize);
                }
                1
            }
            EnumChildWindows(parent, Some(owner_draw), 0);
        }
        // K3: the main and Appearance windows use the menu font. 10 pt JetBrains Mono
        // is about as wide as the old 16 px Segoe UI, so the fixed layouts still fit.
        let font = fonts::create(&config.menu_font, 10, false, 96);
        unsafe extern "system" fn set_font(hwnd: HWND, parameter: LPARAM) -> i32 {
            SendMessageW(hwnd, WM_SETFONT, parameter as usize, 1);
            1
        }
        EnumChildWindows(main, Some(set_font), font as isize);
        EnumChildWindows(help, Some(set_font), font as isize);
        EnumChildWindows(launcher, Some(set_font), font as isize);
        EnumChildWindows(appearance, Some(set_font), font as isize);
        let mut tray: NOTIFYICONDATAW = zeroed();
        tray.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        tray.hWnd = main;
        tray.uID = 1;
        tray.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        tray.uCallbackMessage = TRAY_MESSAGE;
        tray.hIcon = app_icon();
        for (i, c) in wide("Omarchy Overlay — paused").iter().enumerate() {
            tray.szTip[i] = *c;
        }
        let mut app = App {
            main,
            launcher,
            edit,
            list,
            status,
            toggle,
            tray,
            font,
            brush,
            appearance,
            appearance_profiles: appearance_ui::create(font, brush),
            appearance_options: appearance_options::create(font, brush),
            bar_pick: (0, 0),
            rule_center: app_rules_ui::create(font, brush),
            rule_windows: vec![],
            appearance_preview: None,
            outline: focus_outline::create(brush),
            osd: osd::create(font, brush),
            taskbar_hidden_preferred: config.taskbar_mode == "hidden",
            pending_system_action: None,
            selected_label,
            opacity_label,
            bar_label,
            inactive_label,
            selected: null_mut(),
            opacity: opacity::Engine::new(&directory),
            frames: window_theme::Engine::new(&directory),
            settings: settings::Panel::new(font, brush),
            routed: HashMap::new(),
            config,
            directory,
            profile: active_profile,
            profiles: profiles_ui::create(font, brush),
            bar_settings: bar_settings::create(font, brush),
            module_settings: module_settings::create(font, brush),
            tasks: tasks::Panel::new(font, brush),
            custom_app: None,
            help,
            moves: HashMap::new(),
            active: false,
            motions: HashMap::new(),
            tiled: HashMap::new(),
            dwindle: HashMap::new(),
            last_tiled_focus: None,
            layout_preferences,
            layout_all_desktops: false,
            layout_monitor: "*".into(),
            layout_desktop: String::new(),
            scratchpads: HashMap::new(),
            menu_target: None,
            layout_sessions: HashMap::new(),
            drag: None,
            order: vec![],
            floating: HashSet::new(),
            saved: HashMap::new(),
            scrolling: HashMap::new(),
            signature: vec![],
            dirty: true,
            entries: collect_apps(),
            filtered: vec![],
            command_menu: false,
            picker_icons: HashMap::new(),
            picker_history: fs::read_to_string(data_directory().join("app-picker.json"))
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default(),
            hotkeys: vec![],
            cycle_binding: None,
            cycle_error: None,
            failures: vec![],
            binds: config_files.binds,
            menus: config_files.menus,
            webapps: config_files.webapps,
            config_errors: config_files.errors,
            overlay_undo: None,
            confirm_choice: None,
            keys_paused: false,
            keep_awake: false,
            pending_command: None,
            bind_generation: 0,
            last_profile: None,
        };
        app.refresh_apps();
        app.restart_keys();
        if Shell_NotifyIconW(NIM_ADD, &app.tray) == 0 {
            app.failures
                .push("Tray icon unavailable; use the controls window.".into());
        }
        app.filter_apps();
        app.update_appearance();
        let warning = app.failures.join("\r\n");
        APP.with(|cell| *cell.borrow_mut() = Some(app));
        APP.with(|cell| {
            let state = cell.borrow();
            let app = state.as_ref().unwrap();
            app.apply_own_appearance();
            // The saved theme's Windows accent and background return with every start, since exit restores them.
            appearance_effects::apply_theme_effects(
                app.directory.clone(),
                app.config.theme.clone(),
                theme::current().dark,
                theme::windows_accent(&app.config.theme, theme::current()),
            );
            bar::start(
                main,
                &app.directory,
                app.config.show_top_bar,
                app.config.auto_hide_taskbar,
                &app.config.taskbar_mode,
                app.config.bar_opacity,
                &app.config.bar,
            );
        });
        SetTimer(main, 1, 650, None);
        SetTimer(main, 2, 40, None);
        SetTimer(main, STICK_TIMER, 2000, None);
        panel_scroll::fit(main);
        overlay::create();
        APP.with(|cell| {
            if let Some(app) = cell.borrow_mut().as_mut() {
                overlay::set_font(&app.config.menu_font, app.config.menu_font_size);
                app.open_menu("main");
            }
        });
        APP.with(|cell| {
            if let Some(app) = cell.borrow().as_ref() {
                app.update_dashboard();
            }
        });
        if !warning.is_empty() {
            log(&warning);
        }
        APP.with(|cell| {
            if let Some(app) = cell.borrow_mut().as_mut() {
                if app.config.start_tiling {
                    app.toggle();
                    log("Started with tiling on (asInvoker, no startup registration).");
                } else {
                    log("Started paused (Start with tiling on is off; asInvoker, no startup registration).");
                }
            }
        });
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let consumed = APP.with(|cell| {
                let Ok(mut state) = cell.try_borrow_mut() else {
                    return false;
                };
                let Some(app) = state.as_mut() else {
                    return false;
                };
                if msg.message == WM_KEYDOWN {
                    let root = GetAncestor(msg.hwnd, GA_ROOT);
                    if root == app.tasks.hwnd {
                        match msg.wParam as u16 {
                            VK_ESCAPE => {
                                ShowWindow(root, SW_HIDE);
                                return true;
                            }
                            VK_RETURN
                                if [GetDlgItem(root, 301), GetDlgItem(root, 302), root]
                                    .contains(&msg.hwnd) =>
                            {
                                app.tasks.switch();
                                return true;
                            }
                            VK_UP | VK_DOWN if msg.hwnd == GetDlgItem(root, 301) => {
                                let list = GetDlgItem(root, 302);
                                let current = SendMessageW(list, LB_GETCURSEL, 0, 0) as i32;
                                let delta = if msg.wParam as u16 == VK_DOWN { 1 } else { -1 };
                                let next = (current + delta)
                                    .clamp(0, (app.tasks.filtered.len() as i32 - 1).max(0));
                                SendMessageW(list, LB_SETCURSEL, next as usize, 0);
                                return true;
                            }
                            _ => {}
                        }
                    }
                    if [
                        app.main,
                        app.launcher,
                        app.settings.hwnd,
                        app.appearance,
                        app.profiles,
                        app.bar_settings,
                        app.module_settings,
                        app.tasks.hwnd,
                        app.help,
                        app.appearance_profiles,
                        app.appearance_options,
                        app.rule_center,
                    ]
                    .contains(&root)
                    {
                        if msg.wParam as u16 == VK_TAB {
                            let next = GetNextDlgTabItem(
                                root,
                                msg.hwnd,
                                i32::from(GetAsyncKeyState(VK_SHIFT as i32) < 0),
                            );
                            if !next.is_null() {
                                panel_scroll::reveal(root, next);
                                SetFocus(next);
                            }
                            return true;
                        }
                        if msg.wParam as u16 == VK_ESCAPE {
                            PostMessageW(root, WM_CLOSE, 0, 0);
                            return true;
                        }
                        if msg.wParam as u16 == VK_RETURN
                            && window_class(msg.hwnd).eq_ignore_ascii_case("Button")
                        {
                            PostMessageW(
                                root,
                                WM_COMMAND,
                                GetDlgCtrlID(msg.hwnd) as usize,
                                msg.hwnd as isize,
                            );
                            return true;
                        }
                    }
                }
                if msg.message == WM_KEYDOWN
                    && IsWindowVisible(app.launcher) != 0
                    && (msg.hwnd == app.edit || msg.hwnd == app.list || msg.hwnd == app.launcher)
                {
                    match msg.wParam as u16 {
                        VK_ESCAPE => {
                            ShowWindow(app.launcher, SW_HIDE);
                            return true;
                        }
                        VK_RETURN => {
                            app.launch_selected();
                            return true;
                        }
                        VK_DOWN | VK_UP => {
                            let selected = SendMessageW(app.list, LB_GETCURSEL, 0, 0) as i32;
                            let delta = if msg.wParam as u16 == VK_DOWN { 1 } else { -1 };
                            let next =
                                (selected + delta).clamp(0, (app.filtered.len() as i32 - 1).max(0));
                            SendMessageW(app.list, LB_SETCURSEL, next as usize, 0);
                            InvalidateRect(app.list, null(), 0);
                            app.update_picker();
                            return true;
                        }
                        _ => {}
                    }
                }
                false
            });
            if !consumed {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        bar::stop();
        APP.with(|cell| {
            if let Some(mut app) = cell.borrow_mut().take() {
                app.restore();
                if let Err(error) = app.opacity.restore_all() {
                    log(&error);
                }
                if let Err(error) = app.frames.restore_all() {
                    log(&error)
                }
                if let Err(error) = appearance_effects::restore(&app.directory) {
                    log(&error);
                }
                keyhook::stop();
                for id in app.hotkeys {
                    UnregisterHotKey(main, id);
                }
                overlay::destroy();
                if app.keep_awake {
                    windows_sys::Win32::System::Power::SetThreadExecutionState(
                        windows_sys::Win32::System::Power::ES_CONTINUOUS,
                    );
                }
                Shell_NotifyIconW(NIM_DELETE, &app.tray);
                DestroyWindow(launcher);
                DestroyWindow(main);
                DestroyWindow(app.appearance);
                DestroyWindow(app.settings.hwnd);
                DestroyWindow(app.bar_settings);
                DestroyWindow(app.module_settings);
                DestroyWindow(app.tasks.hwnd);
                DestroyWindow(app.help);
                DestroyWindow(app.appearance_profiles);
                DestroyWindow(app.appearance_options);
                DestroyWindow(app.rule_center);
                DestroyWindow(app.outline);
                DestroyWindow(app.osd);
                DestroyWindow(app.profiles);
                DeleteObject(app.font);
                DeleteObject(app.brush);
            }
        });
        fonts::unload();
        CloseHandle(mutex);
        log("Exited; original window placements restored.");
        match EXIT_REASON.load(std::sync::atomic::Ordering::Relaxed) {
            1 => message("The desktop has been restored and Omarchy Overlay has closed.\r\nYou can remove the stick now."),
            2 => message(if ON_STICK.load(std::sync::atomic::Ordering::Relaxed) {
                "OmarchyOverlay.exe is gone: the stick it runs from was removed.\r\nThe taskbar, windows and transparency were put back and Omarchy Overlay closed.\r\nPlug the stick back in and start Omarchy Overlay to carry on."
            } else {
                "OmarchyOverlay.exe is gone: the drive it runs from is no longer there.\r\nThe taskbar, windows and transparency were put back and Omarchy Overlay closed.\r\nReconnect the drive and start Omarchy Overlay to carry on."
            }),
            3 => message(&format!(
                "OmarchyOverlay.exe is gone from its folder, but the drive is still there:\r\n{}\r\n\r\nIt was deleted or moved, or security software (Windows Security or Sophos) removed it.\r\nThe taskbar, windows and transparency were put back and Omarchy Overlay closed.\r\nDon't click Allow or Restore in the security software; ask IT if it was quarantined.",
                std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default()
            )),
            _ => {}
        }
    }
}

impl App {
    unsafe fn show_settings(&mut self) {
        self.settings.refresh(&self.config, self.selected);
        self.sync_cycle_hotkey();
        crate::desktops::follow(self.settings.hwnd);
        panel_scroll::fit(self.settings.hwnd);
        ShowWindow(self.settings.hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(self.settings.hwnd);
    }
    unsafe fn show_bar_settings(&self) {
        bar_settings::fill(self.bar_settings, &self.config.bar);
        bar_settings::fill_taskbar(self.bar_settings, &self.config);
        crate::desktops::follow(self.bar_settings);
        panel_scroll::fit(self.bar_settings);
        ShowWindow(self.bar_settings, SW_SHOWNORMAL);
        SetForegroundWindow(self.bar_settings);
    }
    unsafe fn settings_command(&mut self, id: usize, notification: usize) {
        if id == 60 && notification == CBN_SELCHANGE as usize {
            self.settings.layout_controls();
            return;
        }

        if id == 73 {
            let text = text_of(GetDlgItem(self.settings.hwnd, 71));
            match crate::shortcuts::parse(&text) {
                Ok(_) => {
                    self.cycle_error = None;
                    self.config.cycle_workspace_shortcut = text.trim().into();
                    self.cycle_binding = None;
                    self.sync_cycle_hotkey();
                    self.update_help();
                    self.save_config();
                }
                Err(error) => {
                    set_text(GetDlgItem(self.settings.hwnd, 74), &error);
                    self.cycle_error = Some(error);
                }
            }
            return;
        }
        if id == 75 {
            set_text(
                GetDlgItem(self.settings.hwnd, 71),
                crate::shortcuts::DEFAULT_CYCLE,
            );
            return;
        }
        if id == 69 {
            self.show_bar_settings();
            return;
        }
        if id == 68 {
            self.show_profiles();
            return;
        }
        match id {
            76 => self.show_app_rules(),
            42 if notification == LBN_SELCHANGE as usize => {
                self.settings.fill(
                    self.settings
                        .selected_index()
                        .and_then(|i| self.config.rules.get(i)),
                );
            }
            50 => {
                if IsWindow(self.selected) == 0 {
                    message("Focus a regular application window, then reopen Settings.");
                    return;
                }
                let exe = process_name(self.selected);
                let index = self.config.rules.iter().position(|r| {
                    r.executable.eq_ignore_ascii_case(&exe)
                        && r.title_contains.is_empty()
                        && r.window_class.is_empty()
                });
                SendMessageW(
                    GetDlgItem(self.settings.hwnd, 42),
                    LB_SETCURSEL,
                    index.unwrap_or(usize::MAX),
                    0,
                );
                let rule = index
                    .and_then(|i| self.config.rules.get(i))
                    .cloned()
                    .unwrap_or(AppRule {
                        executable: exe,
                        ..AppRule::default()
                    });
                self.settings.fill(Some(&rule));
            }
            51 => match self.settings.read_rule() {
                Ok(mut rule) => {
                    let index = self
                        .settings
                        .selected_index()
                        .filter(|i| *i < self.config.rules.len())
                        .or_else(|| {
                            self.config.rules.iter().position(|r| {
                                r.executable.eq_ignore_ascii_case(&rule.executable)
                                    && r.title_contains.eq_ignore_ascii_case(&rule.title_contains)
                                    && r.window_class.eq_ignore_ascii_case(&rule.window_class)
                            })
                        });
                    if let Some(i) = index {
                        let old = &self.config.rules[i];
                        rule.active_opacity = old.active_opacity;
                        rule.inactive_opacity = old.inactive_opacity;
                        rule.coordinate_theme = old.coordinate_theme;
                        rule.backdrop = old.backdrop.clone();
                    }
                    let count = self.save_app_rule(index, rule.clone());
                    let selected_exe = process_name(self.selected);
                    let mismatch = if !selected_exe.is_empty()
                        && !selected_exe.eq_ignore_ascii_case(&rule.executable)
                    {
                        format!(" Selected window is {selected_exe}; this rule targets a different app.")
                    } else {
                        String::new()
                    };
                    set_text(
                        GetDlgItem(self.settings.hwnd, 65),
                        &format!(
                            "Saved {} for {} — {count} matching window(s).{mismatch}",
                            rule.mode.label(),
                            rule.executable
                        ),
                    );
                }
                Err(error) => message(&error),
            },
            52 => {
                if let Some(index) = self
                    .settings
                    .selected_index()
                    .filter(|i| *i < self.config.rules.len())
                {
                    self.config.rules.remove(index);
                    self.apply_rules();
                }
            }
            53 => {
                SendMessageW(
                    GetDlgItem(self.settings.hwnd, 42),
                    LB_SETCURSEL,
                    usize::MAX,
                    0,
                );
                self.settings.fill(None);
            }
            54 => {
                for rule in rules::work_starters() {
                    if !self
                        .config
                        .rules
                        .iter()
                        .any(|r| r.executable.eq_ignore_ascii_case(&rule.executable))
                    {
                        self.config.rules.push(rule);
                    }
                }
                self.apply_rules();
            }
            64 => match self.settings.general() {
                Ok((layout, gap, ratio, modifiers)) => {
                    log(&format!("Layout selected in Settings: {layout}"));
                    self.config.layout = layout;
                    self.config.scrolling_columns = text_of(GetDlgItem(self.settings.hwnd, 2340))
                        .parse::<usize>()
                        .unwrap_or(2)
                        .clamp(1, 4);
                    self.config.gap = gap;
                    self.config.main_ratio = ratio;
                    if modifiers != self.config.modifiers {
                        for id in self.hotkeys.drain(..) {
                            UnregisterHotKey(self.main, id);
                        }
                        self.config.modifiers = modifiers;
                        self.failures.clear();
                        self.register_hotkeys();
                        self.sync_cycle_hotkey();
                        self.update_help();
                        if !self.failures.is_empty() {
                            log(&self.failures.join("\r\n"));
                        }
                    }
                    self.save_config();
                    self.dirty = true;
                    self.tick();
                }
                Err(error) => message(&error),
            },
            67 if !shell(&self.profile.path, "") => {
                message("Could not open config.json in its default editor.");
            }
            _ => {}
        }
    }
    unsafe fn apply_rules(&mut self) {
        let release: Vec<_> = self
            .saved
            .keys()
            .copied()
            .filter(|key| {
                let hwnd = *key as HWND;
                self.config
                    .excluded_processes
                    .iter()
                    .any(|name| process_name(hwnd).eq_ignore_ascii_case(name))
                    || window_policy(&self.config, hwnd).mode != Mode::Tile
            })
            .collect();
        for key in release {
            self.restore_one(key);
            self.routed.remove(&key);
        }
        self.appearance_poll();
        self.save_config();
        self.dirty = true;
        self.tick();
        self.settings.refresh(&self.config, self.selected);
    }
    unsafe fn snapshot(&mut self, hwnd: HWND) -> bool {
        let key = hwnd as isize;
        if self
            .saved
            .get(&key)
            .is_some_and(|saved| saved.matches(hwnd))
        {
            return true;
        }
        let mut placement: WINDOWPLACEMENT = zeroed();
        placement.length = size_of::<WINDOWPLACEMENT>() as u32;
        if GetWindowPlacement(hwnd, &mut placement) == 0 {
            log("Skipped a window whose original placement could not be saved.");
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if SetPropW(
            hwnd,
            wide(PLACEMENT_OWNER).as_ptr(),
            GetCurrentProcessId() as usize as HANDLE,
        ) == 0
        {
            log("Could not mark a window for safe placement restoration.");
            return false;
        }
        let mut normal_screen = placement.rcNormalPosition;
        let (dx, dy) = workspace_offset(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST));
        OffsetRect(&mut normal_screen, dx, dy);
        self.saved.insert(
            key,
            Saved {
                placement,
                pid,
                normal_screen,
                marker: GetCurrentProcessId() as usize,
            },
        );
        if let Err(error) = self.save_positions() {
            self.saved.remove(&key);
            RemovePropW(hwnd, wide(PLACEMENT_OWNER).as_ptr());
            log(&error);
            return false;
        }
        true
    }
    unsafe fn route(&mut self, hwnd: HWND, displays: &[settings::Display]) -> bool {
        let key = hwnd as isize;
        let Some(device) = rule_for(&self.config, hwnd).and_then(|r| r.monitor.clone()) else {
            self.routed.remove(&key);
            return false;
        };
        let Some(target) = displays.iter().find(|d| d.device == device) else {
            return false;
        };
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let current = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if let Some(route) = self
            .routed
            .get_mut(&key)
            .filter(|r| r.pid == pid && r.device == device)
        {
            if route.pending == 0 {
                return false;
            }
            if current == target.monitor {
                route.pending = 0;
                return false;
            }
            route.pending -= 1;
            if route.pending == 0 {
                log("Preferred monitor move did not complete; leaving the window on its current monitor.");
                return false;
            }
            return true;
        }
        if current == target.monitor {
            self.routed.insert(
                key,
                Route {
                    pid,
                    device,
                    pending: 0,
                },
            );
            return false;
        }
        let Some(source) = displays.iter().find(|d| d.monitor == current) else {
            return false;
        };
        if !self.snapshot(hwnd) {
            return false;
        }
        let mut rect: RECT = zeroed();
        GetWindowRect(hwnd, &mut rect);
        let target_rect = layout::transfer(
            Rect {
                x: rect.left,
                y: rect.top,
                w: rect.right - rect.left,
                h: rect.bottom - rect.top,
            },
            source.work,
            target.work,
        );
        if IsZoomed(hwnd) != 0 {
            ShowWindowAsync(hwnd, SW_RESTORE);
        }
        position(hwnd, target_rect);
        self.routed.insert(
            key,
            Route {
                pid,
                device,
                pending: 4,
            },
        );
        true
    }
    unsafe fn update_appearance(&self) {
        set_text(
            self.bar_label,
            &format!("Top bar opacity: {}%", self.config.bar_opacity),
        );
        set_text(
            self.inactive_label,
            &if self.config.inactive_opacity == 100 {
                "Inactive window opacity: off (100%)".into()
            } else {
                format!("Inactive window opacity: {}%", self.config.inactive_opacity)
            },
        );
        if IsWindow(self.selected) != 0 {
            set_text(
                self.selected_label,
                &format!("Selected: {}", text_of(self.selected)),
            );
            let percent = self.opacity.percent(self.selected);
            set_text(
                self.opacity_label,
                &format!("Selected window opacity: {percent}%"),
            );
            SendMessageW(
                GetDlgItem(self.appearance, 33),
                TBM_SETPOS,
                1,
                percent as isize,
            );
        } else {
            set_text(self.selected_label, "Focus an app window to select it");
            set_text(self.opacity_label, "No app window selected");
            SendMessageW(GetDlgItem(self.appearance, 33), TBM_SETPOS, 1, 100);
        }
    }
    unsafe fn show_appearance(&mut self) {
        self.update_appearance();
        crate::desktops::follow(self.appearance);
        ShowWindow(self.appearance, SW_SHOWNORMAL);
        SetForegroundWindow(self.appearance);
    }
    fn save_config(&self) {
        if let Err(error) = profile::save(&self.profile, &self.directory, &self.config) {
            unsafe {
                message(&format!(
                    "Preferences applied for this session, but settings could not be saved: {error}"
                ));
            }
        }
    }
    unsafe fn apply_theme(&mut self, name: &str) {
        self.config.theme = name.into();
        theme::set(name);
        let old = self.brush;
        self.brush = CreateSolidBrush(theme::current().background);
        let dark: i32 = i32::from(theme::current().dark);
        for hwnd in [
            self.main,
            self.launcher,
            self.appearance,
            self.settings.hwnd,
            self.profiles,
            self.bar_settings,
            self.module_settings,
            self.tasks.hwnd,
            self.help,
            self.appearance_profiles,
            self.appearance_options,
            self.rule_center,
            self.outline,
            self.osd,
        ] {
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, self.brush as isize);
            DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
            theme_children(hwnd, dark != 0);
            RedrawWindow(
                hwnd,
                null(),
                null_mut(),
                RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME,
            );
        }
        DeleteObject(old);
        self.apply_own_appearance();
        appearance_effects::apply_theme_effects(
            self.directory.clone(),
            name.to_string(),
            theme::current().dark,
            theme::windows_accent(name, theme::current()),
        );
        bar::appearance(self.config.bar_opacity);
        if self
            .directory
            .join("terminal-appearance-restore.json")
            .exists()
        {
            if let Err(error) = appearance_effects::apply_terminal(&self.directory, name) {
                log(&error);
            }
        }
        self.save_config();
    }
    unsafe fn appearance_poll(&mut self) {
        let foreground = GetForegroundWindow();
        let mut windows: Vec<HWND> = vec![];
        unsafe extern "system" fn enumerate(hwnd: HWND, data: LPARAM) -> i32 {
            if basic_eligible(hwnd) {
                (*(data as *mut Vec<HWND>)).push(hwnd);
            }
            1
        }
        EnumWindows(Some(enumerate), &mut windows as *mut _ as isize);
        if basic_eligible(foreground) && !elevated(foreground) && foreground != self.selected {
            self.selected = foreground;
            if IsWindowVisible(self.appearance) != 0 {
                self.update_appearance();
            }
        }
        let frame_windows = windows.clone();
        windows.retain(|&hwnd| !elevated(hwnd) && can_change_opacity(&self.config, hwnd));
        self.opacity
            .enforce_rules(|hwnd| can_change_opacity(&self.config, hwnd));
        let config = &self.config;
        self.frames.poll(&frame_windows, foreground, |h| {
            let p = window_policy(config, h);
            let excluded = config
                .excluded_processes
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&process_name(h)));
            (p.mode != Mode::Ignore
                && (config.window_borders
                    || p.coordinate_theme
                    || p.material_explicit && p.backdrop != 0)
                && !excluded
                && !elevated(h))
            .then_some((
                p.coordinate_theme,
                // Borders alone must not apply the global material (pre-border behaviour).
                if p.coordinate_theme || p.material_explicit {
                    p.backdrop
                } else {
                    0
                },
                config.window_borders,
            ))
        });
        let config = &self.config;
        self.opacity.poll(&windows, foreground, |h| {
            let p = window_policy(config, h);
            (p.active_opacity, p.inactive_opacity)
        });
    }
    unsafe fn slider(&mut self, sender: HWND, persist: bool) {
        let percent = SendMessageW(sender, TBM_GETPOS, 0, 0) as u8;
        match GetDlgCtrlID(sender) {
            31 => {
                self.config.bar_opacity = percent.clamp(50, 100);
                bar::appearance(self.config.bar_opacity);
            }
            32 => {
                self.config.inactive_opacity = percent.clamp(70, 100);
                self.appearance_poll();
            }
            33 => {
                if IsWindow(self.selected) == 0 {
                    message("Focus a regular application window first, then reopen Appearance.");
                    return;
                }
                if !basic_eligible(self.selected) || elevated(self.selected) {
                    message("The selected window is now minimized, full-screen, or protected. Focus a regular app window before changing its opacity.");
                    return;
                }
                if !can_change_opacity(&self.config, self.selected) {
                    message("This app's rule keeps its native opacity. Change the rule in Settings to allow opacity.");
                    return;
                }
                if let Err(error) = self.opacity.manual(self.selected, percent) {
                    message(&error);
                }
            }
            _ => return,
        }
        self.update_appearance();
        if persist && GetDlgCtrlID(sender) != 33 {
            self.save_config();
        }
    }
    unsafe fn opacity_hotkey(&mut self, index: u32) {
        let hwnd = GetForegroundWindow();
        if !basic_eligible(hwnd) || elevated(hwnd) || !can_change_opacity(&self.config, hwnd) {
            return;
        }
        self.selected = hwnd;
        let result = if index == 26 {
            self.opacity.manual(hwnd, 100)
        } else {
            let old = i32::from(self.opacity.percent(hwnd));
            let percent = (old + if index == 24 { -5 } else { 5 }).clamp(50, 100) as u8;
            self.opacity.manual(hwnd, percent)
        };
        if let Err(error) = result {
            message(&error);
        }
        self.update_appearance();
    }
    unsafe fn register_hotkeys(&mut self) {
        self.cycle_binding = None;
        self.cycle_error = None;
        for attempt in 0..2 {
            if attempt > 0 {
                // A controller that was just killed or is still exiting can hold every chord briefly.
                self.failures
                    .retain(|f| !f.starts_with("Shortcut unavailable"));
                Sleep(1000);
            }
            let mut occupied = HashSet::new();
            for (index, action) in crate::shortcuts::ACTIONS.iter().enumerate() {
                let chord = crate::shortcuts::chord(
                    index,
                    &self.config.modifiers,
                    &self.config.shortcut_overrides,
                );
                let Some(binding) = crate::shortcuts::parse(&chord).ok().flatten() else {
                    continue;
                };
                let id = BASE as i32 + crate::shortcuts::action_slot(index) as i32;
                let reason = if !occupied.insert((binding.modifiers, binding.key)) {
                    "another action uses it".to_string()
                } else if RegisterHotKey(
                    self.main,
                    id,
                    binding.modifiers | MOD_NOREPEAT,
                    binding.key,
                ) != 0
                {
                    self.hotkeys.push(id);
                    continue;
                } else {
                    format!("Windows error {}", GetLastError())
                };
                self.failures.push(format!("Shortcut unavailable: {} ({chord}). Another action, app or Windows may own it ({reason}).", action.label));
            }
            if !self.hotkeys.is_empty()
                || !self
                    .failures
                    .iter()
                    .any(|f| f.starts_with("Shortcut unavailable"))
            {
                break;
            }
        }
    }
    unsafe fn rebind_hotkeys(&mut self) {
        for id in self.hotkeys.drain(..) {
            UnregisterHotKey(self.main, id);
        }
        self.failures.clear();
        self.register_hotkeys();
        self.sync_cycle_hotkey();
        self.update_help();
    }
    unsafe fn update_help(&self) {
        shortcuts_ui::fill(self, None);
    }
    unsafe fn sync_cycle_hotkey(&mut self) {
        let registered = self.hotkeys.contains(&CYCLE_HOTKEY);
        if self.keys_paused || self.config.keymap == "omarchy" {
            if registered {
                UnregisterHotKey(self.main, CYCLE_HOTKEY);
                self.hotkeys.retain(|&id| id != CYCLE_HOTKEY);
            }
            self.cycle_binding = None;
            return;
        }
        let binding = crate::shortcuts::parse(&self.config.cycle_workspace_shortcut)
            .ok()
            .flatten();
        // A cached observed desktop list is refreshed by the existing 650 ms bar poll.
        // Release this shortcut if observation fails or cycling would do nothing.
        let available = crate::desktops::snapshot().is_some_and(|s| s.ids.len() > 1);
        let status = if binding.is_none() {
            "Workspace cycling disabled (blank shortcut).".to_string()
        } else if !available {
            "Shortcut inactive: at least two observable Windows desktops are needed.".into()
        } else {
            String::new()
        };
        // Rebind only when the desired chord changes. Avoid unregistering on each timer tick.
        let desired = binding.filter(|_| available);
        if self.cycle_binding != desired || (registered && desired.is_none()) {
            if registered {
                UnregisterHotKey(self.main, CYCLE_HOTKEY);
                self.hotkeys.retain(|&id| id != CYCLE_HOTKEY);
            }
            self.cycle_binding = desired;
            if let Some(binding) = desired {
                if self.cycle_conflicts(binding)
                    || RegisterHotKey(
                        self.main,
                        CYCLE_HOTKEY,
                        binding.modifiers | MOD_NOREPEAT,
                        binding.key,
                    ) == 0
                {
                    set_text(GetDlgItem(self.settings.hwnd, 74), &format!(
                        "Unavailable: {}. Another action, app or Windows owns it; choose another shortcut.",
                        self.config.cycle_workspace_shortcut));
                    return;
                }
                self.hotkeys.push(CYCLE_HOTKEY);
            }
        }
        if let Some(error) = &self.cycle_error {
            set_text(GetDlgItem(self.settings.hwnd, 74), error);
        } else if !status.is_empty() {
            set_text(GetDlgItem(self.settings.hwnd, 74), &status);
        } else if self.hotkeys.contains(&CYCLE_HOTKEY) {
            set_text(
                GetDlgItem(self.settings.hwnd, 74),
                &format!(
                    "Ready: {} cycles native desktops on all monitors; last → first.",
                    self.config.cycle_workspace_shortcut
                ),
            );
        }
    }
    unsafe fn cycle_conflicts(&self, binding: crate::shortcuts::Binding) -> bool {
        crate::shortcuts::conflict(binding, crate::shortcuts::ACTIONS.len(), &self.config).is_some()
    }
    unsafe fn toggle(&mut self) {
        if self.active {
            self.restore();
            self.active = false;
            if !self.saved.is_empty() {
                self.open_level(overlay::Level::Page("restore-failed"), true);
            }
        } else {
            self.active = true;
            self.dirty = true;
            self.tick();
        }
        set_text(
            self.status,
            if self.active {
                "ACTIVE — tiling ordinary app windows on each monitor"
            } else {
                "PAUSED — original window placements restored"
            },
        );
        set_text(
            self.toggle,
            if self.active {
                "Pause and restore"
            } else {
                "Enable tiling"
            },
        );
        self.tray.szTip.fill(0);
        let tip = if self.active {
            "Omarchy Overlay — tiling active"
        } else {
            "Omarchy Overlay — paused"
        };
        for (i, c) in wide(tip).iter().enumerate() {
            self.tray.szTip[i] = *c;
        }
        Shell_NotifyIconW(NIM_MODIFY, &self.tray);
        log(if self.active {
            "Tiling enabled."
        } else {
            "Paused; original placements restored."
        });
        keyhook::TILING.store(self.active, std::sync::atomic::Ordering::Relaxed);
        bar::set_active(self.active);
    }
    unsafe fn restore(&mut self) {
        self.finish_motion(false);
        self.restore_scratchpads();
        self.tiled.clear();
        self.dwindle.clear();
        self.last_tiled_focus = None;
        self.layout_sessions.clear();
        self.layout_desktop.clear();
        self.drag = None;
        self.moves.clear();
        self.saved.retain(|&key, saved| !saved.restore(key as HWND));
        if let Err(error) = self.save_positions() {
            log(&error);
        }
        self.order.clear();
        self.floating.clear();
        self.signature.clear();
        self.routed.clear();
    }
    fn save_positions(&self) -> Result<(), String> {
        recovery::save_positions(&self.directory, &self.saved)
    }
    unsafe fn restore_one(&mut self, key: isize) {
        if self
            .saved
            .get(&key)
            .is_some_and(|saved| saved.restore(key as HWND))
        {
            self.saved.remove(&key);
        }
        if let Err(error) = self.save_positions() {
            log(&error);
        }
    }
    unsafe fn tick(&mut self) {
        if !self.active || self.drag.is_some() || crate::desktops::switching() {
            return;
        }
        if let Some(desktop) = workspace_ui::current_desktop() {
            if desktop != self.layout_desktop {
                if !self.layout_desktop.is_empty() {
                    self.layout_sessions.insert(
                        self.layout_desktop.clone(),
                        (
                            std::mem::take(&mut self.tiled),
                            std::mem::take(&mut self.dwindle),
                            std::mem::take(&mut self.order),
                        ),
                    );
                }
                let (tiles, trees, order) =
                    self.layout_sessions.remove(&desktop).unwrap_or_default();
                self.tiled = tiles;
                self.dwindle = trees;
                self.order = order;
                self.layout_desktop = desktop;
                self.last_tiled_focus = None;
                if let Some(s) = crate::desktops::snapshot() {
                    self.layout_sessions.retain(|key, _| {
                        s.ids
                            .iter()
                            .any(|id| crate::workspace_layouts::desktop_key(id) == *key)
                    });
                }
                self.signature.clear();
                self.dirty = true;
            }
        }
        self.release_scrolling_for_other_layouts();
        let mut found: Vec<isize> = vec![];
        unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> i32 {
            if basic_eligible(hwnd) || scrolling_runtime::current_hidden(hwnd) {
                (*(parameter as *mut Vec<isize>)).push(hwnd as isize);
            }
            1
        }
        EnumWindows(Some(enumerate), &mut found as *mut _ as isize);
        found.retain(|&key| {
            if scrolling_runtime::owned(key as HWND)
                && !self
                    .saved
                    .get(&key)
                    .is_some_and(|saved| saved.matches(key as HWND))
            {
                return false;
            }
            if self
                .scratchpads
                .values()
                .any(|pad| pad.task.hwnd as isize == key && pad.task.valid())
            {
                return false;
            }
            let hwnd = key as HWND;
            let name = process_name(hwnd);
            !self
                .config
                .excluded_processes
                .iter()
                .any(|excluded| name.eq_ignore_ascii_case(excluded))
                && !elevated(hwnd)
                && window_policy(&self.config, hwnd).mode == Mode::Tile
        });
        self.routed.retain(|&key, route| {
            let mut pid = 0;
            GetWindowThreadProcessId(key as HWND, &mut pid);
            IsWindow(key as HWND) != 0 && pid == route.pid
        });
        let displays = settings::displays();
        found.retain(|&key| {
            !self.pending_move(key as HWND, &displays) && !self.route(key as HWND, &displays)
        });
        let saved_count = self.saved.len();
        self.saved.retain(|&key, saved| saved.matches(key as HWND));
        if self.saved.len() != saved_count {
            if let Err(error) = self.save_positions() {
                log(&error);
            }
        }
        self.floating.retain(|&key| IsWindow(key as HWND) != 0);
        self.order
            .retain(|key| found.contains(key) && !self.floating.contains(key));
        // Reverse z-order makes existing windows enter in a stable, bottom-to-top order.
        for &key in found.iter().rev() {
            if !self.order.contains(&key) && !self.floating.contains(&key) {
                self.order.push(key);
            }
        }
        let signature: Vec<_> = self
            .order
            .iter()
            .filter_map(|&key| {
                let monitor = MonitorFromWindow(key as HWND, MONITOR_DEFAULTTONEAREST);
                let mut info: MONITORINFO = zeroed();
                info.cbSize = size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(monitor, &mut info) == 0 {
                    return None;
                }
                let r = info.rcWork;
                Some((
                    key,
                    monitor as isize,
                    Rect {
                        x: r.left,
                        y: r.top,
                        w: r.right - r.left,
                        h: r.bottom - r.top,
                    },
                ))
            })
            .collect();
        let foreground = GetForegroundWindow() as isize;
        if self.tiled.values().any(|g| g.keys.contains(&foreground)) {
            self.last_tiled_focus = Some(foreground);
        }
        if self.order.contains(&foreground) {
            let monitor = MonitorFromWindow(foreground as HWND, MONITOR_DEFAULTTONEAREST);
            if self.layout_for_monitor(monitor) == "scrolling" {
                let state = self
                    .scrolling
                    .entry((self.layout_desktop.clone(), monitor as isize))
                    .or_default();
                if state.active != foreground {
                    state.active = foreground;
                    self.dirty = true;
                }
            }
        }
        if signature == self.signature && !self.dirty {
            return;
        }
        self.signature = signature;
        self.dirty = false;
        let monitors: HashSet<_> = self
            .signature
            .iter()
            .map(|(_, monitor, _)| *monitor)
            .collect();
        self.tiled.retain(|monitor, _| monitors.contains(monitor));
        self.dwindle.retain(|monitor, _| monitors.contains(monitor));
        for monitor in monitors {
            let group: Vec<_> = self
                .signature
                .iter()
                .filter(|(_, m, _)| *m == monitor)
                .copied()
                .collect();
            let Some((_, _, area)) = group.first() else {
                continue;
            };
            let area = &layout::outer(*area, self.config.gap, self.config.outer_gap);
            let mode = self.layout_for_monitor(monitor as HMONITOR);
            if mode == "scrolling" {
                self.tile_scrolling(monitor, &group, *area);
                continue;
            }
            if mode == "dwindle" {
                let gap = self.config.gap;
                let keys = group.iter().map(|(key, _, _)| *key).collect::<Vec<_>>();
                let tree = self.dwindle.entry(monitor).or_default();
                tree.sync(&keys, self.last_tiled_focus, *area, gap);
                let placements = tree.rects(*area, gap);
                self.tiled.insert(
                    monitor,
                    resize::TiledGroup {
                        keys: placements.iter().map(|(k, _)| *k).collect(),
                        area: *area,
                        mode: "dwindle".into(),
                        gap,
                        ratio: self.config.main_ratio,
                        rects: placements.iter().map(|(_, r)| *r).collect(),
                    },
                );
                for key in keys {
                    if !placements.iter().any(|(k, _)| *k == key) {
                        self.restore_one(key);
                    }
                }
                for (key, target) in placements {
                    let hwnd = key as HWND;
                    if !self.snapshot(hwnd) {
                        continue;
                    }
                    if IsZoomed(hwnd) != 0 {
                        ShowWindowAsync(hwnd, SW_RESTORE);
                        self.dirty = true;
                    }
                    self.arrange_window(hwnd, target);
                }
                continue;
            }
            // Leave space for sensible minimum window sizes instead of piling up tiny panes.
            let gap = self.config.gap;
            let max_stack = ((area.h - 2 * gap) / (180 + gap).max(1)).max(1) as usize;
            let maximum = match mode.as_str() {
                "columns" => ((area.w - 2 * gap) / (320 + gap).max(1)).max(1) as usize,
                "rows" => max_stack,
                "grid" => layout::grid_capacity(*area, gap),
                _ => max_stack + 1,
            };
            let count = group.len().min(maximum);
            for (key, _, _) in group.iter().skip(count) {
                self.restore_one(*key);
            }
            let keys = group
                .iter()
                .take(count)
                .map(|(key, _, _)| *key)
                .collect::<Vec<_>>();
            let cached = self
                .tiled
                .entry(monitor)
                .or_insert_with(|| resize::TiledGroup {
                    keys: vec![],
                    area: *area,
                    mode: String::new(),
                    gap,
                    ratio: self.config.main_ratio,
                    rects: vec![],
                });
            if cached.keys != keys
                || cached.area != *area
                || cached.mode != mode
                || cached.gap != gap
                || cached.ratio != self.config.main_ratio
            {
                *cached = resize::TiledGroup {
                    keys,
                    area: *area,
                    mode: mode.clone(),
                    gap,
                    ratio: self.config.main_ratio,
                    rects: layout::arrange(*area, count, gap, self.config.main_ratio, &mode),
                };
            }
            let rects = cached.rects.clone();
            for ((key, _, _), target) in group.iter().take(count).zip(rects) {
                let hwnd = *key as HWND;
                if !self.snapshot(hwnd) {
                    continue;
                }
                if IsZoomed(hwnd) != 0 {
                    ShowWindowAsync(hwnd, SW_RESTORE);
                    self.dirty = true;
                }
                self.arrange_window(hwnd, target);
            }
        }
    }
    unsafe fn action(&mut self, index: u32) {
        match index {
            0 => self.toggle(),
            1 => self.open_menu("main"),
            2 => {
                if !shell(Path::new(&self.config.terminal), "") && !shell(Path::new("cmd.exe"), "")
                {
                    message("The terminal could not be opened. Set terminal in config.json to an installed executable.");
                }
            }
            3 => self.float_window(GetForegroundWindow()),
            4 => self.open_level(overlay::Level::Keybinds, true),
            5 => PostQuitMessage(0),
            32 => self.promote(GetForegroundWindow()),
            33 => self.cycle_layout(),
            34 => self.move_monitor(GetForegroundWindow(), -1),
            35 => self.move_monitor(GetForegroundWindow(), 1),
            36 => self.tasks.open(None),
            37 => self.rebalance(),
            39 => self.change_split(GetForegroundWindow(), true, 0.),
            40 => self.move_direction(GetForegroundWindow(), -1, 0),
            41 => self.move_direction(GetForegroundWindow(), 1, 0),
            42 => self.move_direction(GetForegroundWindow(), 0, -1),
            43 => self.move_direction(GetForegroundWindow(), 0, 1),
            44 => self.close_window(GetForegroundWindow()),
            45 => self.maximize_window(GetForegroundWindow()),
            46 => self.resize_keyboard(GetForegroundWindow(), true, -30),
            47 => self.resize_keyboard(GetForegroundWindow(), true, 30),
            48 => self.resize_keyboard(GetForegroundWindow(), false, -30),
            49 => self.resize_keyboard(GetForegroundWindow(), false, 30),
            50 => self.opacity_preset(GetForegroundWindow(), 85),
            51 => self.opacity_preset(GetForegroundWindow(), 90),
            52 => self.opacity_preset(GetForegroundWindow(), 95),
            53 => self.opacity_preset(GetForegroundWindow(), 98),
            54 => self.assign_scratchpad(GetForegroundWindow()),
            55 => self.toggle_scratchpad(),
            56 => self.queue_system_action(0),
            57 => self.queue_system_action(1),
            38 => {
                if crate::desktops::cycle_next() {
                    bar::poll();
                }
                self.sync_cycle_hotkey();
            }
            28..=31 => {
                if !crate::desktops::select((index - 28) as usize) {
                    message("That Windows desktop is unavailable. Open Windows Task View (Win + Tab) to create or select your desktops.");
                }
            }
            24..=26 => self.opacity_hotkey(index),
            27 => {
                self.config.inactive_opacity = 100;
                SendMessageW(GetDlgItem(self.appearance, 32), TBM_SETPOS, 1, 100);
                if let Err(error) = self.opacity.restore_all() {
                    message(&error);
                }
                self.update_appearance();
                self.save_config();
            }
            14 | 15
                if self.layout_for_monitor(MonitorFromWindow(
                    GetForegroundWindow(),
                    MONITOR_DEFAULTTONEAREST,
                )) == "dwindle" =>
            {
                self.change_split(
                    GetForegroundWindow(),
                    false,
                    if index == 14 { -0.05 } else { 0.05 },
                )
            }
            14 | 15 => {
                self.config.main_ratio = (self.config.main_ratio
                    + if index == 14 { -0.05 } else { 0.05 })
                .clamp(0.25, 0.75);
                self.dirty = true;
                self.tick();
            }
            6..=13 | 16..=23 => {
                if !self.active {
                    return;
                }
                let (direction, swap) = if index >= 16 {
                    ((index - 16) % 4, index >= 20)
                } else {
                    ((index - 6) % 4, index >= 10)
                };
                let (dx, dy) = match direction {
                    0 => (-1, 0),
                    1 => (0, 1),
                    2 => (0, -1),
                    _ => (1, 0),
                };
                self.navigate(dx, dy, swap);
            }
            _ => {}
        }
    }
    unsafe fn navigate(&mut self, dx: i32, dy: i32, swap: bool) {
        let current = GetForegroundWindow() as isize;
        if dx != 0
            && self.scroll_step(
                MonitorFromWindow(current as HWND, MONITOR_DEFAULTTONEAREST),
                dx,
                swap,
            )
        {
            return;
        }
        let windows: Vec<_> = self
            .order
            .iter()
            .copied()
            .filter(|key| IsIconic(*key as HWND) == 0)
            .collect();
        let Some(index) = windows.iter().position(|key| *key == current) else {
            return;
        };
        let rects: Vec<_> = windows
            .iter()
            .map(|key| {
                let mut r: RECT = zeroed();
                GetWindowRect(*key as HWND, &mut r);
                Rect {
                    x: r.left,
                    y: r.top,
                    w: r.right - r.left,
                    h: r.bottom - r.top,
                }
            })
            .collect();
        if let Some(next) = layout::directional(&rects, index, dx, dy) {
            let destination = windows[next];
            if swap {
                // Swapping order across monitors would not move either window; keep it local.
                if MonitorFromWindow(current as HWND, MONITOR_DEFAULTTONEAREST)
                    != MonitorFromWindow(destination as HWND, MONITOR_DEFAULTTONEAREST)
                {
                    return;
                }
                let a = self.order.iter().position(|key| *key == current).unwrap();
                let b = self
                    .order
                    .iter()
                    .position(|key| *key == destination)
                    .unwrap();
                let monitor = MonitorFromWindow(current as HWND, MONITOR_DEFAULTTONEAREST) as isize;
                if self.layout_for_monitor(monitor as HMONITOR) == "dwindle" {
                    if let Some(tree) = self.dwindle.get_mut(&monitor) {
                        tree.swap(current, destination);
                    }
                }
                self.order.swap(a, b);
                self.dirty = true;
                self.tick();
            } else {
                SetForegroundWindow(destination as HWND);
            }
        }
    }
    unsafe fn show_launcher(&mut self) {
        self.show_search(true);
    }
    unsafe fn show_search(&mut self, commands: bool) {
        self.command_menu = commands;
        let foreground = GetForegroundWindow();
        if basic_eligible(foreground) && !elevated(foreground) {
            self.selected = foreground;
        }
        // Opening search from our dashboard retains the last external selection.
        self.menu_target = tasks::collect()
            .into_iter()
            .find(|task| task.hwnd == self.selected);
        self.refresh_apps();
        set_text(
            self.launcher,
            if commands {
                "Omarchy Overlay — command menu"
            } else {
                "Omarchy Overlay — find an app"
            },
        );
        let mut pointer: POINT = zeroed();
        GetCursorPos(&mut pointer);
        let monitor = MonitorFromPoint(pointer, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        let r = info.rcWork;
        SetWindowPos(
            self.launcher,
            HWND_TOPMOST,
            r.left + (r.right - r.left - 740) / 2,
            r.top + (r.bottom - r.top - 580) / 3,
            740,
            580,
            SWP_SHOWWINDOW,
        );
        crate::desktops::follow(self.launcher);
        set_text(self.edit, "");
        self.filter_apps();
        SetForegroundWindow(self.launcher);
        SetFocus(self.edit);
    }
    unsafe fn filter_apps(&mut self) {
        let query = text_of(self.edit).to_lowercase();
        let apps = self
            .entries
            .iter()
            .map(|entry| (entry.label.clone(), entry.key()))
            .collect::<Vec<_>>();
        self.filtered = self.picker_history.rank(&query, &apps);
        SendMessageW(self.list, WM_SETREDRAW, 0, 0);
        SendMessageW(self.list, LB_RESETCONTENT, 0, 0);
        for &index in &self.filtered {
            SendMessageW(
                self.list,
                LB_ADDSTRING,
                0,
                wide(&format!(
                    "{}{}",
                    if self.picker_history.favorite(&self.entries[index].key()) {
                        "★  "
                    } else {
                        ""
                    },
                    self.entries[index].label
                ))
                .as_ptr() as isize,
            );
        }
        if self.filtered.is_empty() {
            SendMessageW(
                self.list,
                LB_ADDSTRING,
                0,
                wide("No matching apps").as_ptr() as isize,
            );
        }
        SendMessageW(self.list, LB_SETCURSEL, 0, 0);
        SendMessageW(self.list, WM_SETREDRAW, 1, 0);
        InvalidateRect(self.list, null(), 1);
        self.update_picker();
    }
    unsafe fn update_picker(&self) {
        let selected = SendMessageW(self.list, LB_GETCURSEL, 0, 0) as usize;
        let entry = self
            .filtered
            .get(selected)
            .and_then(|index| self.entries.get(*index));
        EnableWindow(
            GetDlgItem(self.launcher, 12),
            i32::from(entry.is_some_and(Entry::available)),
        );
        EnableWindow(GetDlgItem(self.launcher, 13), i32::from(entry.is_some()));
        set_text(
            GetDlgItem(self.launcher, 12),
            match entry.and_then(|e| e.action.as_ref()) {
                Some(command_menu::Action::Command(_)) => "Run action",
                Some(command_menu::Action::Window(_)) => "Switch window",
                None => "Open app",
            },
        );
        set_text(
            GetDlgItem(self.launcher, 13),
            if entry.is_some_and(|entry| self.picker_history.favorite(&entry.key())) {
                "Unpin favorite"
            } else {
                "Pin favorite"
            },
        );
        let detail = entry
            .map(|entry| {
                if let Some(action) = &entry.action {
                    return match action {
                        command_menu::Action::Command(_) => {
                            "Omarchy Overlay action — Enter to run".into()
                        }
                        command_menu::Action::Window(task) => {
                            format!("Open window — {}", task.identity.label)
                        }
                    };
                }
                if entry.custom_key.is_some() {
                    return format!(
                        "{}: {}",
                        if entry.available() {
                            "Custom app"
                        } else {
                            "Missing app or working folder"
                        },
                        entry.path.display()
                    );
                }
                format!(
                    "{}{}",
                    if self.picker_history.recent(&entry.key()) {
                        "Recently used · "
                    } else {
                        ""
                    },
                    if entry.arguments.starts_with('"') {
                        "Installed Windows app"
                    } else if entry
                        .path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk")
                            || ext.eq_ignore_ascii_case("appref-ms"))
                    {
                        "Start menu shortcut"
                    } else {
                        "Built-in shortcut"
                    }
                )
            })
            .unwrap_or_else(|| "Try a shorter app name or click Refresh apps.".into());
        set_text(GetDlgItem(self.launcher, 15), &detail);
    }
    fn save_picker(&self) {
        if let Err(error) = write_json_atomic(
            &self.directory.join("app-picker.json"),
            &self.picker_history,
        ) {
            log(&format!("App picker preferences: {error}"));
        }
    }
    unsafe fn pin_selected(&mut self) {
        let selected = SendMessageW(self.list, LB_GETCURSEL, 0, 0) as usize;
        if let Some(&index) = self.filtered.get(selected) {
            self.picker_history.toggle(&self.entries[index].key());
            self.save_picker();
            self.filter_apps();
        }
    }
    unsafe fn launch_selected(&mut self) {
        let selected = SendMessageW(self.list, LB_GETCURSEL, 0, 0) as usize;
        let Some(&index) = self.filtered.get(selected) else {
            return;
        };
        let entry = self.entries[index].clone();
        ShowWindow(self.launcher, SW_HIDE);
        if let Some(action) = entry.action.clone() {
            self.picker_history.launched(&entry.key());
            self.save_picker();
            self.run_menu_action(action);
            return;
        }
        if !entry.available() {
            message(&format!("This custom entry is unavailable on this PC. Check its path and working folder in Manage apps.\r\n{}",entry.path.display()));
            return;
        }
        if shell_in(
            &entry.path,
            &entry.arguments,
            entry.working_directory.as_deref(),
        ) {
            self.picker_history.launched(&entry.key());
            self.save_picker();
        } else {
            message(&format!(
                "Could not launch {}. Its shortcut may no longer be valid.",
                entry.label
            ));
        }
    }
    unsafe fn tray_menu(&mut self) {
        let exe = std::env::current_exe().unwrap_or_default();
        let install_items = crate::install::tray_install_items(
            crate::install::admin_marker(&exe),
            crate::install::is_installed(&exe),
        );
        let install_label = if install_items.0 {
            crate::install_files::tray_label(
                crate::install::installed_version().as_deref(),
                env!("CARGO_PKG_VERSION"),
            )
        } else {
            String::new()
        };
        let menu = CreatePopupMenu();
        for (id, label) in [
            (
                1,
                if self.active {
                    "Pause and restore windows"
                } else {
                    "Enable tiling"
                },
            ),
            (2, "Open menu"),
            (
                3,
                if self.keys_paused {
                    "Resume Omarchy Overlay keys"
                } else {
                    "Pause Omarchy Overlay keys"
                },
            ),
            (6, "Open the settings window"),
            (5, "About Omarchy Overlay"),
            (0, ""),
            (8, install_label.as_str()),
            (9, "Uninstall…"),
            (7, "Exit before removing the stick"),
            (4, "Restore desktop and exit"),
        ] {
            if id == 7 && !ON_STICK.load(std::sync::atomic::Ordering::Relaxed) {
                // Only offered when Omarchy Overlay runs from a removable drive.
                continue;
            }
            if (id == 8 && !install_items.0) || (id == 9 && !install_items.1) {
                continue;
            }
            if id == 0 {
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            } else {
                AppendMenuW(menu, MF_STRING, id, wide(label).as_ptr());
            }
        }
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(self.main);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            self.main,
            null(),
        );
        DestroyMenu(menu);
        match selected {
            1 => self.toggle(),
            2 => self.open_menu("main"),
            3 => self.run_command(&crate::commands::Command::Builtin("pause-keys"), false),
            4 => PostQuitMessage(0),
            5 => about(self.main),
            7 => exit_for_stick(),
            8 | 9 => {
                // The new process elevates itself and closes this copy before it changes anything.
                let flag = if selected == 8 { "--install" } else { "--uninstall" };
                if let Err(error) = std::process::Command::new(&exe).arg(flag).spawn() {
                    message(&error.to_string());
                }
            }
            6 => {
                crate::desktops::follow(self.main);
                // K6: opens on the primary monitor like the other settings windows.
                panel_scroll::fit(self.main);
                ShowWindow(self.main, SW_SHOWNORMAL);
                SetForegroundWindow(self.main);
            }
            _ => {}
        }
        PostMessageW(self.main, WM_NULL, 0, 0);
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if window_theme::backdrop_active(hwnd) {
        if msg == WM_ERASEBKGND {
            let mut r: RECT = zeroed();
            GetClientRect(hwnd, &mut r);
            FillRect(w as HDC, &r, GetStockObject(BLACK_BRUSH) as HBRUSH);
            return 1;
        }
        if msg == WM_PAINT {
            let mut p: PAINTSTRUCT = zeroed();
            let dc = BeginPaint(hwnd, &mut p);
            FillRect(dc, &p.rcPaint, GetStockObject(BLACK_BRUSH) as HBRUSH);
            EndPaint(hwnd, &p);
            return 0;
        }
    }

    if msg == WM_PAINT && window_class(hwnd) == "OmarchyOverlay.FocusOutline" {
        let mut paint: PAINTSTRUCT = zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        let brush = CreateSolidBrush(theme::current().accent);
        FillRect(dc, &paint.rcPaint, brush);
        DeleteObject(brush);
        EndPaint(hwnd, &paint);
        return 0;
    }
    if panel_scroll::supports(hwnd) {
        match msg {
            WM_SIZE => panel_scroll::update(hwnd),
            WM_VSCROLL => {
                panel_scroll::scroll(hwnd, (w & 0xffff) as i32);
                return 0;
            }
            WM_MOUSEWHEEL => {
                panel_scroll::scroll(
                    hwnd,
                    if ((w >> 16) as i16) > 0 {
                        SB_LINEUP
                    } else {
                        SB_LINEDOWN
                    },
                );
                return 0;
            }
            _ => {}
        }
    }
    if [
        WM_CTLCOLORSTATIC,
        WM_CTLCOLOREDIT,
        WM_CTLCOLORLISTBOX,
        WM_CTLCOLORBTN,
    ]
    .contains(&msg)
    {
        let palette = theme::current();
        let color = if msg == WM_CTLCOLOREDIT || msg == WM_CTLCOLORLISTBOX {
            palette.surface
        } else {
            palette.background
        };
        SetTextColor(w as HDC, palette.text);
        if msg == WM_CTLCOLORSTATIC && window_theme::backdrop_active(hwnd) {
            SetBkMode(w as HDC, TRANSPARENT as i32);
            return GetStockObject(HOLLOW_BRUSH) as isize;
        }
        SetBkColor(w as HDC, color);
        SetDCBrushColor(w as HDC, color);
        return GetStockObject(DC_BRUSH) as isize;
    }
    if msg == WM_DRAWITEM && l != 0 {
        let item = &*(l as *const DRAWITEMSTRUCT);
        if item.CtlType == ODT_BUTTON {
            let palette = theme::current();
            SetDCBrushColor(item.hDC, palette.surface);
            FillRect(item.hDC, &item.rcItem, GetStockObject(DC_BRUSH) as HBRUSH);
            SetDCBrushColor(
                item.hDC,
                if item.itemState & ODS_FOCUS != 0 {
                    palette.accent
                } else {
                    palette.border
                },
            );
            FrameRect(item.hDC, &item.rcItem, GetStockObject(DC_BRUSH) as HBRUSH);
            SetBkMode(item.hDC, TRANSPARENT as i32);
            SetTextColor(
                item.hDC,
                if item.itemState & ODS_DISABLED != 0 {
                    palette.muted
                } else {
                    palette.text
                },
            );
            let old_font = SelectObject(
                item.hDC,
                SendMessageW(item.hwndItem, WM_GETFONT, 0, 0) as HGDIOBJ,
            );
            let text = wide(&text_of(item.hwndItem));
            let mut rect = item.rcItem;
            DrawTextW(
                item.hDC,
                text.as_ptr(),
                (text.len() - 1) as i32,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(item.hDC, old_font);
            return 1;
        }
    }
    let handled = APP.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            return None;
        };
        let app = state.as_mut()?;
        match msg {
            WM_DRAWITEM if l != 0 && hwnd == app.launcher => {
                let item = &*(l as *const DRAWITEMSTRUCT);
                if item.CtlType == ODT_LISTBOX && item.CtlID == 11 {
                    app.draw_picker(item);
                    Some(1)
                } else {
                    None
                }
            }
            m if m == WM_APP + 41 => {
                bar::enforce_hidden();
                Some(0)
            }
            OSD_MESSAGE => {
                app.feedback_message(w);
                Some(0)
            }
            WM_TIMER if w == STICK_TIMER => {
                check_stick();
                Some(0)
            }
            WM_TIMER if w == 4 => {
                app.motion_tick();
                Some(0)
            }
            WM_TIMER if w == 3 => {
                ShowWindow(app.osd, SW_HIDE);
                KillTimer(app.main, 3);
                Some(0)
            }
            WM_TIMER if w == 2 => {
                app.resize_tick();
                Some(0)
            }
            WM_TIMER => {
                bar::poll();
                app.sync_cycle_hotkey();
                app.appearance_poll();
                focus_outline::update(app);
                app.poll_system_action();
                app.tick();
                app.tasks.refresh();
                app.update_dashboard();
                Some(0)
            }
            WM_HOTKEY => {
                app.action(w as u32 - BASE);
                Some(0)
            }
            WM_COMMAND => {
                let id = w & 0xffff;
                let notification = (w >> 16) & 0xffff;
                if hwnd == app.main {
                    match id {
                        1 => app.toggle(),
                        2 => app.show_launcher(),
                        3 => {
                            app.show_settings();
                        }
                        4 => PostQuitMessage(0),
                        6 => bar::toggle_visible(),
                        7 => app.toggle_windows_taskbar(),
                        8 => app.show_appearance(),
                        5 => app.show_help(),
                        10 => {
                            shell(Path::new("explorer.exe"), "");
                        }
                        11 => crate::desktops::windows_panel(b'A' as u16),
                        12 => app.tasks.open(None),
                        13 => crate::desktops::task_view(),
                        14 => app.show_bar_settings(),
                        15 => app.cycle_layout(),
                        16 => app.promote(app.selected),
                        17 => app.float_window(app.selected),
                        18 => app.move_monitor(app.selected, -1),
                        19 => app.move_monitor(app.selected, 1),
                        20 => crate::desktops::windows_panel(b'N' as u16),
                        25 => app.action(14),
                        26 => app.action(15),
                        27 => app.rebalance(),
                        28 => about(app.main),
                        29 => app.show_workspace_layouts(),
                        30 => app.show_bar_editor(),
                        31 => app.show_appearance_profiles(),
                        32 => app.show_appearance_options(),
                        33 => app.show_app_rules(),
                        34 => app.show_applications(),
                        35 => app.show_profiles(),
                        _ => {}
                    }
                } else if hwnd == app.rule_center {
                    app_rules_ui::command(app, id, notification);
                } else if hwnd == app.appearance_options {
                    appearance_options::command(app, id);
                } else if hwnd == app.appearance_profiles {
                    appearance_ui::command(app, id, notification);
                } else if hwnd == app.help {
                    shortcuts_ui::command(app, id, notification);
                } else if hwnd == app.tasks.hwnd {
                    app.tasks.command(id, notification);
                } else if hwnd == app.bar_settings {
                    if bar_settings::choice_command(hwnd, id, notification) {
                        return Some(0);
                    }
                    if id == 212 {
                        // Defaults apply at once, like Restore defaults in the menu.
                        bar_settings::fill(hwnd, &crate::bar_model::BarConfig::default());
                    }
                    match id {
                        211 | 212 => match bar_settings::read(hwnd, &app.config.bar) {
                            Ok(config) => {
                                app.config.bar = config;
                                app.save_config();
                                bar::settings(&app.config.bar);
                                let mode = bar_settings::taskbar_choice(hwnd);
                                if mode != "legacy" && mode != app.config.taskbar_mode {
                                    app.set_taskbar_policy(mode);
                                }
                                app.dirty = true;
                            }
                            Err(error) => message(&error),
                        },
                        213 => crate::desktops::task_view(),
                        215 => app.show_workspace_layouts(),
                        223 => {
                            let previous = app.config.bar.clone();
                            app.config.bar.show_open_windows();
                            if let Err(error) = app.config.bar.validate() {
                                app.config.bar = previous;
                                message(&error);
                                return Some(0);
                            }
                            app.save_config();
                            bar::settings(&app.config.bar);
                            bar_settings::fill(hwnd, &app.config.bar);
                        }
                        218 => app.show_bar_editor(),
                        214 => {
                            module_settings::fill(app.module_settings, &app.config.bar);
                            crate::desktops::follow(app.module_settings);
                            panel_scroll::fit(app.module_settings);
                            ShowWindow(app.module_settings, SW_SHOWNORMAL);
                            SetForegroundWindow(app.module_settings);
                        }
                        _ => {}
                    }
                } else if hwnd == app.module_settings {
                    match id {
                        454..=456 => {
                            let mode = match id {
                                454 => crate::bar_model::DisplayMode::Icon,
                                455 => crate::bar_model::DisplayMode::IconText,
                                _ => crate::bar_model::DisplayMode::Text,
                            };
                            for (i, _) in crate::bar_model::DISPLAY_MODULES.iter().enumerate() {
                                SendMessageW(
                                    GetDlgItem(hwnd, 401 + i as i32),
                                    CB_SETCURSEL,
                                    match mode {
                                        crate::bar_model::DisplayMode::Text => 0,
                                        crate::bar_model::DisplayMode::IconText => 1,
                                        _ => 2,
                                    },
                                    0,
                                );
                            }
                        }
                        451 => {
                            module_settings::apply(hwnd, &mut app.config.bar);
                            app.save_config();
                            bar::settings(&app.config.bar);
                        }
                        452 => {
                            // Defaults apply at once, like Restore defaults in the menu.
                            module_settings::fill(hwnd, &crate::bar_model::BarConfig::default());
                            module_settings::apply(hwnd, &mut app.config.bar);
                            app.save_config();
                            bar::settings(&app.config.bar);
                        }
                        453 => {
                            ShowWindow(hwnd, SW_HIDE);
                        }
                        _ => {}
                    }
                } else if hwnd == app.profiles {
                    if let Err(error) = app.profiles_command(id) {
                        message(&format!("Profile operation failed: {error}"));
                    }
                } else if hwnd == app.settings.hwnd {
                    app.settings_command(id, notification);
                } else if hwnd == app.appearance {
                    match id {
                        38 => app.show_appearance_profiles(),
                        30 if notification == CBN_SELCHANGE as usize => {
                            let index =
                                SendMessageW(GetDlgItem(hwnd, 30), CB_GETCURSEL, 0, 0) as usize;
                            if let Some((name, _)) = theme::NAMES.get(index) {
                                app.apply_theme(name);
                            }
                        }
                        35 => {
                            if let Err(error) = app.opacity.manual(app.selected, 100) {
                                message(&error);
                            }
                            app.update_appearance();
                        }
                        36 => {
                            app.config.inactive_opacity = 100;
                            SendMessageW(GetDlgItem(hwnd, 32), TBM_SETPOS, 1, 100);
                            if let Err(error) = app.opacity.restore_all() {
                                message(&error);
                            }
                            app.update_appearance();
                            app.save_config();
                        }
                        _ => {}
                    }
                } else if id == 10 && notification == EN_CHANGE as usize {
                    app.filter_apps();
                } else if hwnd == app.launcher {
                    match id {
                        11 if notification == LBN_DBLCLK as usize => app.launch_selected(),
                        11 if notification == LBN_SELCHANGE as usize => app.update_picker(),
                        12 => app.launch_selected(),
                        13 => app.pin_selected(),
                        16 => app.show_applications(),
                        14 => {
                            app.refresh_apps();
                        }
                        _ => {}
                    }
                }
                Some(0)
            }
            WM_HSCROLL if hwnd == app.appearance => {
                app.slider(l as HWND, (w & 0xffff) as u32 != TB_THUMBTRACK);
                Some(0)
            }
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                Some(0)
            }
            WM_QUERYENDSESSION => {
                keyhook::stop();
                app.restore();
                if let Err(error) = app.opacity.restore_all() {
                    log(&error);
                }
                if let Err(error) = app.frames.restore_all() {
                    log(&error)
                }
                if let Err(error) = appearance_effects::restore(&app.directory) {
                    log(&error);
                }
                bar::stop();
                Some(1)
            }
            n if n == WM_APP + 59 => {
                message(&bar::tray_status());
                Some(0)
            }
            BAR_SCROLL => {
                app.scroll_step(l as HMONITOR, if w == 0 { -1 } else { 1 }, false);
                Some(0)
            }
            BAR_ACTION => {
                match w {
                    0 => app.open_level(overlay::Level::Apps, true),
                    1 => app.toggle(),
                    2 => app.open_menu("main"),
                    3 => bar::toggle_visible(),
                    63 => exit_for_stick(),
                    4 => app.toggle_windows_taskbar(),
                    5 => PostQuitMessage(0),
                    6 => app.open_menu("style"),
                    7 => app.open_menu("setup"),
                    8 => app.open_menu("bar"),
                    9 => crate::desktops::task_view(),
                    10 => app
                        .tasks
                        .open(if l == 0 { None } else { Some(l as HMONITOR) }),
                    11 => {
                        shell(Path::new("explorer.exe"), "");
                    }
                    12 => crate::desktops::windows_panel(b'A' as u16),
                    13 => crate::desktops::windows_panel(b'N' as u16),
                    15 => {
                        shell(Path::new("ms-settings:"), "");
                    }
                    40..=42 => app.set_layout(["main-stack", "columns", "rows"][w - 40]),
                    47 => app.set_layout("grid"),
                    48 => app.rebalance(),
                    49 => app.set_layout("dwindle"),
                    51 => app.set_layout("scrolling"),
                    60 => app.set_taskbar_policy("visible"),
                    61 => app.set_taskbar_policy("auto-hide"),
                    62 => app.set_taskbar_policy("hidden"),
                    200 => app.queue_system_action(4),
                    201 => app.queue_system_action(3),
                    202 => {
                        shell(Path::new("ms-settings:display"), "");
                    }
                    203 => app.open_menu("system"),
                    204 => app.queue_system_action(0),
                    205 => app.queue_system_action(1),
                    50 => app.change_split(app.selected, true, 0.),
                    43 => app.promote(app.selected),
                    44 => app.float_window(app.selected),
                    45 => app.move_monitor(app.selected, -1),
                    46 => app.move_monitor(app.selected, 1),
                    _ => {}
                }
                Some(0)
            }
            TRAY_MESSAGE => {
                if l as u32 == WM_RBUTTONUP {
                    app.tray_menu();
                } else if l as u32 == WM_LBUTTONUP {
                    SetForegroundWindow(app.main);
                    app.open_menu("main");
                }
                Some(0)
            }
            keyhook::BIND_MESSAGE => {
                if l as usize == app.bind_generation && !app.keys_paused {
                    if let Some(command) = app.binds.get(w).map(|b| b.command.clone()) {
                        app.run_command(&command, false);
                    }
                }
                Some(0)
            }
            keyhook::TAP_MESSAGE => {
                app.open_level(overlay::Level::Apps, true);
                Some(0)
            }
            command_run::PENDING_COMMAND => {
                if let Some((command, confirmed)) = app.pending_command.take() {
                    app.run_command(&command, confirmed);
                }
                Some(0)
            }
            _ => None,
        }
    });
    handled.unwrap_or_else(|| DefWindowProcW(hwnd, msg, w, l))
}

unsafe fn basic_eligible(hwnd: HWND) -> bool {
    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 || GetWindowTextLengthW(hwnd) == 0 {
        return false;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() {
        return false;
    }
    if !GetWindow(hwnd, GW_OWNER).is_null() {
        return false;
    }
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let extended = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if style & WS_THICKFRAME == 0 || extended & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) != 0 {
        return false;
    }
    let mut class = [0u16; 256];
    let len = GetClassNameW(hwnd, class.as_mut_ptr(), 256);
    let class = String::from_utf16_lossy(&class[..len as usize]);
    if [
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "Progman",
        "WorkerW",
        "#32770",
        "Windows.UI.Core.CoreWindow",
    ]
    .contains(&class.as_str())
    {
        return false;
    }
    let mut cloaked: u32 = 0;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut _ as _,
        size_of::<u32>() as u32,
    );
    if cloaked != 0 {
        return false;
    }
    let mut r: RECT = zeroed();
    if GetWindowRect(hwnd, &mut r) == 0 {
        return false;
    }
    if r.right - r.left < 200 || r.bottom - r.top < 120 {
        return false;
    }
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(monitor, &mut info);
    // Borderless full-screen games and video must keep their own geometry.
    if r.left <= info.rcMonitor.left
        && r.top <= info.rcMonitor.top
        && r.right >= info.rcMonitor.right
        && r.bottom >= info.rcMonitor.bottom
        && IsZoomed(hwnd) == 0
    {
        return false;
    }
    true
}
unsafe fn process_path(hwnd: HWND) -> Option<PathBuf> {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return None;
    }
    let mut path = [0u16; 2048];
    let mut length = path.len() as u32;
    let ok = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length);
    CloseHandle(process);
    (ok != 0).then(|| PathBuf::from(String::from_utf16_lossy(&path[..length as usize])))
}
unsafe fn process_name(hwnd: HWND) -> String {
    process_path(hwnd)
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default()
}
/// Explorer's own taskbar. Other programs (Zebar) make hidden windows with the same
/// class, and FindWindow can return theirs first. Falls back to the first match when
/// no Shell_TrayWnd belongs to explorer.exe, as before 0.20.
unsafe fn explorer_taskbar() -> HWND {
    let class = wide("Shell_TrayWnd");
    let mut first: HWND = null_mut();
    let mut h = FindWindowExW(null_mut(), null_mut(), class.as_ptr(), null());
    while !h.is_null() {
        if process_name(h).eq_ignore_ascii_case("explorer.exe") {
            return h;
        }
        if first.is_null() {
            first = h;
        }
        h = FindWindowExW(null_mut(), h, class.as_ptr(), null());
    }
    first
}
unsafe fn elevated(hwnd: HWND) -> bool {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return true;
    }
    let mut token = null_mut();
    let mut elevation: TOKEN_ELEVATION = zeroed();
    let mut length = 0;
    let result = OpenProcessToken(process, TOKEN_QUERY, &mut token) != 0
        && GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut c_void,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        ) != 0;
    if !token.is_null() {
        CloseHandle(token);
    }
    CloseHandle(process);
    !result || elevation.TokenIsElevated != 0
}
unsafe fn position(hwnd: HWND, target: Rect) {
    if IsIconic(hwnd) != 0 {
        return;
    }
    let mut outer: RECT = zeroed();
    let mut visible: RECT = zeroed();
    GetWindowRect(hwnd, &mut outer);
    let has_visible = DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS as u32,
        &mut visible as *mut _ as _,
        size_of::<RECT>() as u32,
    ) == 0;
    let (left, top, right, bottom) = if has_visible {
        (
            visible.left - outer.left,
            visible.top - outer.top,
            outer.right - visible.right,
            outer.bottom - visible.bottom,
        )
    } else {
        (0, 0, 0, 0)
    };
    // DWM may still report the previous frame immediately after an asynchronous move.
    // Only small, nonnegative frame insets can represent an invisible resize border.
    let limit = (GetDpiForWindow(hwnd).max(96) as i32 * 32 / 96).min(128);
    let (left, top, right, bottom) = if [left, top, right, bottom]
        .iter()
        .all(|value| (0..=limit).contains(value))
    {
        (left, top, right, bottom)
    } else {
        (0, 0, 0, 0)
    };
    if SetWindowPos(
        hwnd,
        null_mut(),
        target.x - left,
        target.y - top,
        target.w + left + right,
        target.h + top + bottom,
        SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS,
    ) == 0
    {
        log("Could not position one window; it may have closed or refused the request.");
    }
}

fn collect_apps() -> Vec<Entry> {
    fn walk(directory: &Path, depth: usize, entries: &mut Vec<Entry>) {
        if depth > 8 {
            return;
        }
        let Ok(items) = fs::read_dir(directory) else {
            return;
        };
        for item in items.flatten() {
            let path = item.path();
            if item
                .file_type()
                .map(|kind| kind.is_symlink())
                .unwrap_or(true)
            {
                continue;
            }
            if path.is_dir() {
                walk(&path, depth + 1, entries);
            } else if path.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("appref-ms")
            }) {
                let label = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                entries.push(Entry {
                    label,
                    path,
                    arguments: String::new(),
                    custom_key: None,
                    working_directory: None,
                    action: None,
                });
            }
        }
    }
    let mut entries = vec![
        Entry {
            label: "File Explorer".into(),
            path: "explorer.exe".into(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: None,
        },
        Entry {
            label: "Notepad".into(),
            path: "notepad.exe".into(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: None,
        },
        Entry {
            label: "Command Prompt".into(),
            path: "cmd.exe".into(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: None,
        },
        Entry {
            label: "Windows Settings".into(),
            path: "ms-settings:".into(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: None,
        },
        Entry {
            label: "Default Browser".into(),
            path: "https://www.google.com".into(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: None,
        },
    ];
    for variable in ["APPDATA", "PROGRAMDATA"] {
        if let Some(root) = std::env::var_os(variable) {
            walk(
                &PathBuf::from(root).join("Microsoft/Windows/Start Menu/Programs"),
                0,
                &mut entries,
            );
        }
    }
    for (label, target) in unsafe { native_status::applications() } {
        let target = if target.starts_with("::")
            || target.starts_with("shell:")
            || Path::new(&target).is_absolute()
        {
            target
        } else {
            format!("shell:AppsFolder\\{target}")
        };
        entries.push(Entry {
            label,
            path: "explorer.exe".into(),
            arguments: format!("\"{}\"", target.replace('"', "")),
            custom_key: None,
            working_directory: None,
            action: None,
        });
    }
    entries.sort_by_key(|e| e.label.to_lowercase());
    crate::webapps::dedupe(
        entries,
        |e| e.label.as_str(),
        |e| crate::webapps::browser_app(&e.path, &e.arguments),
        |e| e.label.push_str(" (web app)"),
    )
}
