//! Super keybinds from binds.conf through a low-level keyboard hook on its own thread.
//! RegisterHotKey cannot take most Super keys from Windows, so the hook watches for the
//! bound keys only, swallows them and tells the main window. Every other key passes
//! through untouched, and stopping the hook gives every key back to Windows.
//! Windows does not pass keys typed into programs running as administrator to this
//! hook, so binds do nothing while such a window is in front.
use super::*;
use crate::binds::{self, ALT, CTRL, SHIFT, SUPER};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Mutex,
};

pub(super) const BIND_MESSAGE: u32 = WM_APP + 70;
pub(super) const TAP_MESSAGE: u32 = WM_APP + 71;
/// Mirrors App::active so tiling-only binds pass through while tiling is off.
pub(super) static TILING: AtomicBool = AtomicBool::new(false);
/// An unassigned key code. Pressing it between Super down and Super up stops Start
/// from opening, which is how Windows tells a Super tap from a Super shortcut.
const MASK: u16 = 0xE8;

struct Bound {
    index: usize,
    repeats: bool,
    tiling: bool,
}
struct State {
    owner: isize,
    keys: HashMap<(u32, u32), Bound>,
    /// Keys whose key-down was swallowed; their key-up is swallowed too.
    held: HashSet<u32>,
    tap: Tap,
    super_down: bool,
    /// Another key was pressed while Super was down, so it was not a tap.
    other: bool,
    generation: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tap {
    Start,
    Picker,
    Nothing,
}
static STATE: Mutex<Option<State>> = Mutex::new(None);
static THREAD: Mutex<Option<(u32, std::thread::JoinHandle<()>)>> = Mutex::new(None);
static GENERATION: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn is_super(vk: u32) -> bool {
    vk == VK_LWIN as u32 || vk == VK_RWIN as u32
}
fn is_modifier(vk: u32) -> bool {
    is_super(vk)
        || [
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
        ]
        .iter()
        .any(|m| *m as u32 == vk)
}
unsafe fn modifiers() -> u32 {
    let down = |vk: VIRTUAL_KEY| GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0;
    let mut bits = 0;
    if down(VK_LWIN) || down(VK_RWIN) {
        bits |= SUPER;
    }
    if down(VK_CONTROL) {
        bits |= CTRL;
    }
    if down(VK_MENU) {
        bits |= ALT;
    }
    if down(VK_SHIFT) {
        bits |= SHIFT;
    }
    bits
}
unsafe fn send(keys: &[(u16, bool)]) {
    let inputs: Vec<INPUT> = keys
        .iter()
        .map(|&(vk, up)| {
            let mut flags = if up { KEYEVENTF_KEYUP } else { 0 };
            if vk == VK_LWIN || vk == VK_RWIN {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: 0,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            }
        })
        .collect();
    SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        size_of::<INPUT>() as i32,
    );
}

/// Decides one key event. True swallows it.
unsafe fn handle(message: u32, vk: u32) -> bool {
    let mut guard = state();
    let Some(s) = guard.as_mut() else {
        return false;
    };
    let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    if is_super(vk) {
        if down {
            if !s.super_down {
                s.super_down = true;
                s.other = false;
            }
            return false;
        }
        let tap = s.super_down && !s.other;
        s.super_down = false;
        if tap && s.tap != Tap::Start {
            if s.tap == Tap::Picker {
                PostMessageW(s.owner as HWND, TAP_MESSAGE, 0, 0);
            }
            drop(guard);
            send(&[(MASK, false), (MASK, true), (vk as u16, true)]);
            return true;
        }
        return false;
    }
    if is_modifier(vk) {
        return false;
    }
    if !down {
        return s.held.remove(&vk);
    }
    if s.super_down {
        s.other = true;
    }
    let mods = modifiers();
    let Some(bound) = s.keys.get(&(mods, vk)) else {
        return false;
    };
    if bound.tiling && !TILING.load(Ordering::Relaxed) {
        return false;
    }
    let repeat = !s.held.insert(vk);
    if !repeat || bound.repeats {
        PostMessageW(
            s.owner as HWND,
            BIND_MESSAGE,
            bound.index,
            s.generation as isize,
        );
    }
    drop(guard);
    if !repeat && mods & (SUPER | ALT) != 0 {
        // Without this, releasing Super opens Start and releasing Alt opens the
        // menu bar of the window in front.
        send(&[(MASK, false), (MASK, true)]);
    }
    true
}

unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = &*(l as *const KBDLLHOOKSTRUCT);
        if k.flags & LLKHF_INJECTED == 0 && handle(w as u32, k.vkCode) {
            return 1;
        }
    }
    CallNextHookEx(null_mut(), code, w, l)
}

/// Starts or restarts the hook with these binds. The generation returned is sent
/// with every BIND_MESSAGE so a key pressed before a reload is ignored.
pub(super) unsafe fn start(owner: HWND, binds: &[binds::Bind], tap: &str) -> Result<usize, String> {
    stop();
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let keys = binds
        .iter()
        .enumerate()
        .map(|(index, b)| {
            (
                (b.key.modifiers, b.key.vk),
                Bound {
                    index,
                    repeats: b.command.repeats(),
                    tiling: b.command.tiling(),
                },
            )
        })
        .collect();
    *state() = Some(State {
        owner: owner as isize,
        keys,
        held: HashSet::new(),
        tap: match tap {
            "picker" => Tap::Picker,
            "nothing" => Tap::Nothing,
            _ => Tap::Start,
        },
        super_down: false,
        other: false,
        generation,
    });
    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let mut msg: MSG = zeroed();
        // Creates this thread's message queue before its id is handed out.
        PeekMessageW(&mut msg, null_mut(), WM_USER, WM_USER, PM_NOREMOVE);
        let h = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), GetModuleHandleW(null()), 0);
        let _ = sender.send((GetCurrentThreadId(), !h.is_null(), GetLastError()));
        if h.is_null() {
            return;
        }
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
        UnhookWindowsHookEx(h);
    });
    match receiver.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok((id, true, _)) => {
            *THREAD.lock().unwrap_or_else(|e| e.into_inner()) = Some((id, thread));
            Ok(generation)
        }
        Ok((_, false, error)) => {
            let _ = thread.join();
            *state() = None;
            Err(format!(
                "Windows did not allow the Super key hook (error {error})."
            ))
        }
        Err(_) => {
            *state() = None;
            Err("The Super key hook did not start in time.".into())
        }
    }
}

/// Removes the hook. Every key goes straight to Windows again.
pub(super) unsafe fn stop() {
    if let Some((id, thread)) = THREAD.lock().unwrap_or_else(|e| e.into_inner()).take() {
        PostThreadMessageW(id, WM_QUIT, 0, 0);
        let _ = thread.join();
    }
    *state() = None;
}

pub(super) fn running() -> bool {
    THREAD.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// Windows' own Super shortcuts that a Omarchy Overlay bind takes over while the hook runs.
pub(super) const WINDOWS_KEYS: &[(&str, &str)] = &[
    ("Super+A", "Quick Settings"),
    ("Super+B", "Select the taskbar tray"),
    ("Super+C", "Copilot"),
    ("Super+D", "Show desktop"),
    ("Super+E", "File Explorer"),
    ("Super+F", "Feedback Hub"),
    ("Super+H", "Voice typing"),
    ("Super+I", "Settings"),
    ("Super+J", "Windows tips"),
    ("Super+K", "Cast"),
    ("Super+M", "Minimize all windows"),
    ("Super+N", "Notifications"),
    ("Super+P", "Project to a screen"),
    ("Super+R", "Run"),
    ("Super+S", "Search"),
    ("Super+T", "Taskbar apps"),
    ("Super+U", "Accessibility settings"),
    ("Super+V", "Clipboard history"),
    ("Super+W", "Widgets"),
    ("Super+X", "Quick Link menu"),
    ("Super+Z", "Snap layouts"),
    ("Super+Comma", "Peek at the desktop"),
    ("Super+Period", "Emoji panel"),
    ("Super+Tab", "Task View"),
    ("Super+Space", "Switch keyboard language"),
    ("Super+Escape", "Close Magnifier"),
    ("Super+Minus", "Magnifier zoom out"),
    ("Super+Equal", "Magnifier zoom in"),
    ("Super+Left", "Snap left"),
    ("Super+Right", "Snap right"),
    ("Super+Up", "Maximize"),
    ("Super+Down", "Restore or minimize"),
    ("Super+Shift+S", "Snipping Tool"),
    ("Super+Shift+M", "Restore minimized windows"),
    ("Super+Shift+Left", "Move window to the left monitor"),
    ("Super+Shift+Right", "Move window to the right monitor"),
    ("Super+Shift+Up", "Stretch window to full height"),
    ("Super+Ctrl+Left", "Previous desktop"),
    ("Super+Ctrl+Right", "Next desktop"),
    ("Super+Ctrl+D", "New desktop"),
    ("Super+Ctrl+C", "Color filters"),
    ("Super+Ctrl+Return", "Narrator"),
    ("Super+Ctrl+N", "Narrator settings"),
    ("Super+Alt+B", "HDR on or off"),
    ("Super+Alt+R", "Game Bar recording"),
    ("Super+1", "Taskbar app 1"),
    ("Super+2", "Taskbar app 2"),
    ("Super+3", "Taskbar app 3"),
    ("Super+4", "Taskbar app 4"),
    ("Super+5", "Taskbar app 5"),
    ("Super+6", "Taskbar app 6"),
    ("Super+7", "Taskbar app 7"),
    ("Super+8", "Taskbar app 8"),
    ("Super+9", "Taskbar app 9"),
    ("Super+Shift+1", "New window of taskbar app 1"),
    ("Super+Shift+2", "New window of taskbar app 2"),
    ("Super+Shift+3", "New window of taskbar app 3"),
    ("Super+Shift+4", "New window of taskbar app 4"),
    ("Super+Shift+5", "New window of taskbar app 5"),
    ("Super+Shift+6", "New window of taskbar app 6"),
    ("Super+Shift+7", "New window of taskbar app 7"),
    ("Super+Shift+8", "New window of taskbar app 8"),
    ("Super+Shift+9", "New window of taskbar app 9"),
];

/// The Windows shortcut a key replaces, if any.
pub(super) fn replaces(key: binds::Key) -> Option<&'static str> {
    let label = key.label();
    WINDOWS_KEYS
        .iter()
        .find(|(k, _)| *k == label)
        .map(|(_, what)| *what)
}

/// Running programs that commonly hold keyboard shortcuts, named so the user knows
/// which one to close or edit. Windows does not say which program owns a key.
pub(super) unsafe fn holders() -> Vec<String> {
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        let found = &mut *(l as *mut Vec<String>);
        let class = window_class(h);
        let entry = if class == "AutoHotkey" {
            let title = text_of(h);
            let script = title.split(" - AutoHotkey").next().unwrap_or(&title).trim();
            Some(format!("AutoHotkey script {script}"))
        } else {
            let name = process_name(h);
            let lower = name.to_ascii_lowercase();
            {
                [
                    ("powertoys", "PowerToys (Keyboard Manager, PowerToys Run)"),
                    ("flow.launcher", "Flow Launcher"),
                    ("everything", "Everything search"),
                    ("ditto", "Ditto clipboard"),
                    ("sharex", "ShareX"),
                    ("greenshot", "Greenshot"),
                    ("komorebi", "komorebi"),
                    ("whkd", "whkd"),
                    ("glazewm", "GlazeWM"),
                ]
                .iter()
                .find(|(part, _)| lower.contains(part))
                .map(|(_, label)| format!("{label} ({name})"))
            }
        };
        if let Some(entry) = entry {
            if !found.contains(&entry) {
                found.push(entry);
            }
        }
        1
    }
    let mut found: Vec<String> = vec![];
    EnumWindows(Some(each), &mut found as *mut _ as LPARAM);
    found
}
