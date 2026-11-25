//! Omarchy's drop-down console (Super + `, default/hypr/qconsole.lua): a terminal that
//! drops over the top half of the screen and goes away again with the same key. The
//! first press starts the terminal; its window becomes the console and floats. Hidden
//! means minimized, so a crash never loses it.
use super::*;
use std::time::{Duration, Instant};

pub(super) struct Console {
    hwnd: HWND,
    pid: u32,
}
/// A terminal started for the console whose window has not appeared yet.
pub(super) struct Pending {
    since: Instant,
    before: HashSet<isize>,
}

unsafe fn top_level() -> HashSet<isize> {
    workspaces::top_windows().into_iter().collect()
}
/// The top half of the work area of the monitor under the pointer, inside the gaps.
unsafe fn place(hwnd: HWND, gap: i32) {
    let mut point: POINT = zeroed();
    GetCursorPos(&mut point);
    let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return;
    }
    let w = info.rcWork;
    let gap = gap.max(0);
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        w.left + gap,
        w.top + gap,
        w.right - w.left - 2 * gap,
        (w.bottom - w.top) / 2 - gap,
        SWP_SHOWWINDOW,
    );
}
impl App {
    fn console_alive(&self) -> bool {
        self.console.as_ref().is_some_and(|c| unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(c.hwnd, &mut pid);
            IsWindow(c.hwnd) != 0 && pid == c.pid
        })
    }
    /// Super + `: shows the console, hides it, or starts it the first time.
    pub(super) unsafe fn toggle_console(&mut self) {
        if !self.console_alive() {
            if let Some(old) = self.console.take() {
                self.floating.remove(&(old.hwnd as isize));
            }
            if self.console_pending.as_ref().is_some_and(|p| p.since.elapsed() < Duration::from_secs(10)) {
                return;
            }
            let before = top_level();
            if !shell(Path::new(&self.config.terminal), "") && !shell(Path::new("cmd.exe"), "") {
                message("The terminal could not be opened. Set terminal in config.json to an installed executable.");
                return;
            }
            self.console_pending = Some(Pending { since: Instant::now(), before });
            return;
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let hwnd = console.hwnd;
        if IsIconic(hwnd) == 0 && GetForegroundWindow() == hwnd {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            ShowWindowAsync(hwnd, SW_MINIMIZE);
        } else {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            place(hwnd, self.config.gap);
            self.focus_or_desktop(Some(hwnd as isize));
        }
    }
    /// From the main timer: adopts the window of a terminal started for the console.
    pub(super) unsafe fn console_tick(&mut self) {
        let Some(pending) = self.console_pending.as_ref() else {
            return;
        };
        if pending.since.elapsed() > Duration::from_secs(10) {
            self.console_pending = None;
            return;
        }
        let found = workspaces::top_windows()
            .into_iter()
            .find(|key| !pending.before.contains(key) && basic_eligible(*key as HWND) && !elevated(*key as HWND));
        let Some(key) = found else {
            return;
        };
        self.console_pending = None;
        let hwnd = key as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        // The console floats: tiling leaves it alone, and the journal restores it on exit.
        self.order.retain(|k| *k != key);
        self.floating.insert(key);
        self.snapshot(hwnd);
        SetPropW(hwnd, wide(window_actions::CONSOLE).as_ptr(), 1 as HANDLE);
        self.console = Some(Console { hwnd, pid });
        place(hwnd, self.config.gap);
        self.focus_or_desktop(Some(key));
        self.dirty = true;
    }
    /// Tiling stopped or the app exits: the console stops being on top.
    pub(super) unsafe fn release_console(&mut self) {
        if let Some(c) = self.console.take() {
            if IsWindow(c.hwnd) != 0 {
                RemovePropW(c.hwnd, wide(window_actions::CONSOLE).as_ptr());
                SetWindowPos(c.hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
        }
        self.console_pending = None;
    }
}
