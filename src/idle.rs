//! Omarchy's idle behavior (config/omarchy/shell.json "idle"): a themed screensaver
//! after a while without input, and locking the PC later still. Both are off unless
//! chosen, since Windows has its own lock timeout. Neither starts while an app keeps the
//! display on (video, a presentation) or while Stay awake is on.
use super::*;
use windows_sys::Win32::System::Power::{CallNtPowerInformation, SystemExecutionState, ES_DISPLAY_REQUIRED};
use windows_sys::Win32::System::Shutdown::LockWorkStation;
use windows_sys::Win32::System::SystemInformation::GetTickCount;

const CLASS: &str = "OmarchyOverlay.Screensaver";
/// Screensaver and lock delays in seconds (0 is off), as the Idle pages offer them.
pub const SCREENSAVER_AFTER: [u32; 5] = [0, 150, 300, 600, 900];
pub const LOCK_AFTER: [u32; 5] = [0, 300, 600, 900, 1800];

pub fn label(seconds: u32) -> String {
    match seconds {
        0 => "Off".into(),
        s if s % 60 == 0 => format!("After {} minutes", s / 60),
        s => format!("After {}:{:02} minutes", s / 60, s % 60),
    }
}
thread_local! { static SAVER: Cell<HWND> = const { Cell::new(null_mut()) }; }
use std::cell::Cell;

/// Milliseconds since the last key press or mouse move anywhere in this session.
unsafe fn idle_ms() -> u32 {
    let mut info = LASTINPUTINFO { cbSize: size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    if GetLastInputInfo(&mut info) == 0 {
        return 0;
    }
    GetTickCount().wrapping_sub(info.dwTime)
}
/// True when some app asked Windows to keep the display on.
unsafe fn display_required() -> bool {
    let mut state = 0u32;
    CallNtPowerInformation(
        SystemExecutionState,
        null(),
        0,
        &mut state as *mut _ as _,
        size_of::<u32>() as u32,
    ) == 0
        && state & ES_DISPLAY_REQUIRED != 0
}
unsafe fn show_saver() {
    static CLASS_DONE: std::sync::Once = std::sync::Once::new();
    CLASS_DONE.call_once(|| {
        let name = wide(CLASS);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&class);
    });
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay screensaver").as_ptr(),
        WS_POPUP,
        GetSystemMetrics(SM_XVIRTUALSCREEN),
        GetSystemMetrics(SM_YVIRTUALSCREEN),
        GetSystemMetrics(SM_CXVIRTUALSCREEN),
        GetSystemMetrics(SM_CYVIRTUALSCREEN),
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if hwnd.is_null() {
        return;
    }
    SAVER.with(|s| s.set(hwnd));
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    ShowCursor(0);
}
unsafe fn close_saver() {
    let hwnd = SAVER.with(|s| s.replace(null_mut()));
    if !hwnd.is_null() {
        DestroyWindow(hwnd);
        ShowCursor(1);
    }
}
unsafe fn paint(hwnd: HWND) {
    let p = theme::current();
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    let mut r: RECT = zeroed();
    GetClientRect(hwnd, &mut r);
    SetDCBrushColor(dc, p.background);
    FillRect(dc, &r, GetStockObject(DC_BRUSH) as HBRUSH);
    let mut time: SYSTEMTIME = zeroed();
    windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time);
    let clock = format!("{:02}:{:02}", time.wHour, time.wMinute);
    let origin = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
    SetBkMode(dc, TRANSPARENT as i32);
    for display in settings::displays() {
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(display.monitor, &mut info) == 0 {
            continue;
        }
        let m = info.rcMonitor;
        let height = (m.bottom - m.top) / 6;
        let font = fonts::create(crate::font_names::NL_FAMILY, height * 72 / 96, true, 96);
        let old = SelectObject(dc, font as HGDIOBJ);
        SetTextColor(dc, p.accent);
        let text = wide(&clock);
        let mut cr = RECT {
            left: m.left - origin.0,
            top: m.top - origin.1,
            right: m.right - origin.0,
            bottom: m.bottom - origin.1,
        };
        DrawTextW(dc, text.as_ptr(), (text.len() - 1) as i32, &mut cr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old);
        DeleteObject(font as HGDIOBJ);
    }
    EndPaint(hwnd, &ps);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_SETCURSOR => {
            SetCursor(null_mut());
            1
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
impl App {
    /// Called from the main timer: starts the screensaver or locks the PC when idle,
    /// and closes the screensaver as soon as there is input again.
    pub(super) unsafe fn idle_tick(&mut self) {
        let idle = idle_ms();
        let shown = !SAVER.with(Cell::get).is_null();
        if idle < 1000 {
            if shown {
                close_saver();
            }
            self.idle_locked = false;
            return;
        }
        if self.keep_awake || display_required() {
            return;
        }
        let saver = self.config.screensaver_after;
        if saver > 0 && !shown && idle >= saver * 1000 {
            show_saver();
        } else if shown {
            InvalidateRect(SAVER.with(Cell::get), null(), 0);
        }
        let lock = self.config.lock_after;
        if lock > 0 && !self.idle_locked && idle >= lock * 1000 {
            self.idle_locked = true;
            close_saver();
            LockWorkStation();
        }
    }
}
