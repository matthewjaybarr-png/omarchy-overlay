//! The bar clock's calendar, as Omarchy's clock opens one: a month in the theme's colors
//! under the clock, today in the accent color. The mouse wheel or the arrows change the
//! month, a click on the title goes back to today, and "Notifications" opens Windows'
//! own panel. Escape or a click anywhere else closes it.
use super::*;
use crate::{calendar, desktops};
use windows_sys::Win32::{
    Globalization::{GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK, LOCALE_SMONTHNAME1, LOCALE_SSHORTESTDAYNAME1},
    System::SystemInformation::{GetLocalTime, GetTickCount},
    UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
};

const CLASS: &str = "OmarchyOverlay.Calendar";
/// Layout at 96 DPI.
const PAD: i32 = 14;
const CELL_W: i32 = 38;
const CELL_H: i32 = 30;
const HEADER: i32 = 40;
const FOOTER: i32 = 36;
const WIDTH: i32 = PAD * 2 + CELL_W * 7;
const HEIGHT: i32 = PAD + HEADER + CELL_H * 7 + FOOTER;

struct Calendar {
    hwnd: HWND,
    family: String,
    dpi: u32,
    year: i32,
    month: u32,
    today: (i32, u32, u32),
    first_day: u32,
    days: [String; 7],
    months: [String; 12],
}
thread_local! {
    static CALENDAR: RefCell<Option<Calendar>> = const { RefCell::new(None) };
    /// When the calendar last closed itself because the user clicked elsewhere: a click on
    /// the clock just then closed it, and must not open it again.
    static CLOSED_AT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

unsafe fn locale(kind: u32) -> String {
    let mut buffer = [0u16; 80];
    let n = GetLocaleInfoEx(null(), kind, buffer.as_mut_ptr(), buffer.len() as i32);
    String::from_utf16_lossy(&buffer[..(n.max(1) - 1) as usize])
}
fn today() -> (i32, u32, u32) {
    let mut time = unsafe { zeroed() };
    unsafe { GetLocalTime(&mut time) };
    (time.wYear as i32, time.wMonth as u32, time.wDay as u32)
}

/// Opens the calendar under `anchor` (the clock, in screen pixels), or closes it.
pub(super) unsafe fn toggle(anchor: RECT, family: &str) {
    if close() || GetTickCount().wrapping_sub(CLOSED_AT.with(|c| c.get())) < 300 {
        return;
    }
    static CLASS_DONE: std::sync::Once = std::sync::Once::new();
    CLASS_DONE.call_once(|| {
        let name = wide(CLASS);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&class);
    });
    let monitor = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(monitor, &mut info);
    let (mut dpi, mut unused) = (96u32, 96u32);
    GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut unused);
    let dpi = dpi.max(96);
    let unit = |v: i32| v * dpi as i32 / 96;
    let (w, h) = (unit(WIDTH), unit(HEIGHT));
    let work = info.rcWork;
    let center = (anchor.left + anchor.right) / 2;
    let x = (center - w / 2).clamp(work.left, (work.right - w).max(work.left));
    // Under a bar at the top, over a bar at the bottom.
    let y = if anchor.top - info.rcMonitor.top < info.rcMonitor.bottom - anchor.bottom {
        anchor.bottom + unit(6)
    } else {
        anchor.top - h - unit(6)
    }
    .clamp(work.top, (work.bottom - h).max(work.top));
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay calendar").as_ptr(),
        WS_POPUP,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if hwnd.is_null() {
        return;
    }
    let corner = 1u32; // DWMWCP_DONOTROUND: square, as Omarchy's menus
    DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &corner as *const _ as _, size_of::<u32>() as u32);
    let today = today();
    let first_day = locale(LOCALE_IFIRSTDAYOFWEEK).trim().parse::<u32>().unwrap_or(0).min(6);
    CALENDAR.with(|c| {
        *c.borrow_mut() = Some(Calendar {
            hwnd,
            family: family.to_string(),
            dpi,
            year: today.0,
            month: today.1,
            today,
            first_day,
            days: std::array::from_fn(|i| locale(LOCALE_SSHORTESTDAYNAME1 + i as u32)),
            months: std::array::from_fn(|i| locale(LOCALE_SMONTHNAME1 + i as u32)),
        })
    });
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

/// Closes the calendar. Returns whether it was open.
pub(super) unsafe fn close() -> bool {
    let Some(hwnd) = CALENDAR.with(|c| c.borrow_mut().take().map(|c| c.hwnd)) else {
        return false;
    };
    DestroyWindow(hwnd);
    true
}

unsafe fn change(delta: i32) {
    let hwnd = CALENDAR.with(|c| {
        c.borrow_mut().as_mut().map(|c| {
            (c.year, c.month) = if delta == 0 { (c.today.0, c.today.1) } else { calendar::step(c.year, c.month, delta) };
            c.hwnd
        })
    });
    if let Some(hwnd) = hwnd {
        InvalidateRect(hwnd, null(), 0);
    }
}

fn blend(over: u32, under: u32, amount: f32) -> u32 {
    let mix = |shift: u32| {
        let a = ((over >> shift) & 255) as f32;
        let b = ((under >> shift) & 255) as f32;
        ((a * amount + b * (1.0 - amount)).round() as u32) << shift
    };
    mix(0) | mix(8) | mix(16)
}

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    CALENDAR.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else { return };
        let p = theme::current();
        let mut r: RECT = zeroed();
        GetClientRect(hwnd, &mut r);
        let dc = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, r.right.max(1), r.bottom.max(1));
        let old_bitmap = SelectObject(dc, bitmap);
        let unit = |v: i32| v * c.dpi as i32 / 96;
        let fill = |rect: &RECT, color: u32| {
            SetDCBrushColor(dc, color);
            FillRect(dc, rect, GetStockObject(DC_BRUSH) as HBRUSH);
        };
        fill(&r, p.background);
        // Omarchy's 2 px border.
        let edge = unit(2).max(1);
        for side in [
            RECT { bottom: edge, ..r },
            RECT { top: r.bottom - edge, ..r },
            RECT { right: edge, ..r },
            RECT { left: r.right - edge, ..r },
        ] {
            fill(&side, p.border);
        }
        SetBkMode(dc, TRANSPARENT as i32);
        let family = if c.family.is_empty() { crate::font_names::NL_FAMILY } else { c.family.as_str() };
        let text = |s: &str, rect: RECT, color: u32, bold: bool, size: i32| {
            let font = fonts::create(family, size, bold, c.dpi);
            let old = SelectObject(dc, font as HGDIOBJ);
            SetTextColor(dc, color);
            let w = wide(s);
            let mut rect = rect;
            DrawTextW(dc, w.as_ptr(), (w.len() - 1) as i32, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            SelectObject(dc, old);
            DeleteObject(font as HGDIOBJ);
        };
        let left = unit(PAD);
        let grid_w = unit(CELL_W * 7);
        let mut y = unit(PAD);
        // Title row: ‹  November 2026  ›
        let row = |y: i32, h: i32, x0: i32, x1: i32| RECT { left: x0, top: y, right: x1, bottom: y + unit(h) };
        let name = c.months.get(c.month as usize - 1).cloned().unwrap_or_default();
        text(&format!("{name} {}", c.year), row(y, HEADER - 8, left + unit(CELL_W), left + grid_w - unit(CELL_W)), p.text, true, 11);
        text("‹", row(y, HEADER - 8, left, left + unit(CELL_W)), p.accent, true, 13);
        text("›", row(y, HEADER - 8, left + grid_w - unit(CELL_W), left + grid_w), p.accent, true, 13);
        y += unit(HEADER);
        let dim = blend(p.text, p.background, 0.55);
        for (column, day) in calendar::weekday_order(c.first_day).into_iter().enumerate() {
            let x = left + unit(CELL_W * column as i32);
            text(&c.days[day], row(y, CELL_H, x, x + unit(CELL_W)), dim, false, 9);
        }
        y += unit(CELL_H);
        for week in calendar::grid(c.year, c.month, c.first_day) {
            for (column, day) in week.into_iter().enumerate() {
                let Some(day) = day else { continue };
                let x = left + unit(CELL_W * column as i32);
                let cell = row(y, CELL_H, x, x + unit(CELL_W));
                let is_today = (c.year, c.month, day) == c.today;
                if is_today {
                    fill(&RECT { left: cell.left + unit(3), right: cell.right - unit(3), top: cell.top + unit(2), bottom: cell.bottom - unit(2) }, p.accent);
                }
                text(&day.to_string(), cell, if is_today { p.background } else { p.text }, is_today, 10);
            }
            y += unit(CELL_H);
        }
        let line = RECT { left, right: left + grid_w, top: y + unit(4), bottom: y + unit(5) };
        fill(&line, blend(p.text, p.background, 0.18));
        text("Notifications", row(y + unit(6), FOOTER - 8, left, left + grid_w), p.accent, false, 10);
        BitBlt(screen, 0, 0, r.right, r.bottom, dc, 0, 0, SRCCOPY);
        SelectObject(dc, old_bitmap);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(dc);
    });
    EndPaint(hwnd, &ps);
}

/// What a click at (x, y), in 96-DPI client units, does.
enum Hit {
    Previous,
    Next,
    Today,
    Notifications,
    Nothing,
}
fn hit(x: i32, y: i32) -> Hit {
    let grid_w = CELL_W * 7;
    if (PAD..PAD + HEADER).contains(&y) {
        match x - PAD {
            v if (0..CELL_W).contains(&v) => Hit::Previous,
            v if (grid_w - CELL_W..grid_w).contains(&v) => Hit::Next,
            v if (CELL_W..grid_w - CELL_W).contains(&v) => Hit::Today,
            _ => Hit::Nothing,
        }
    } else if y >= PAD + HEADER + CELL_H * 7 {
        Hit::Notifications
    } else {
        Hit::Nothing
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEWHEEL => {
            change(if (w >> 16) as i16 > 0 { -1 } else { 1 });
            0
        }
        WM_LBUTTONUP => {
            let dpi = CALENDAR.with(|c| c.borrow().as_ref().map(|c| c.dpi)).unwrap_or(96) as i32;
            let (x, y) = ((l & 0xffff) as i16 as i32 * 96 / dpi, (l >> 16) as i16 as i32 * 96 / dpi);
            match hit(x, y) {
                Hit::Previous => change(-1),
                Hit::Next => change(1),
                Hit::Today => change(0),
                Hit::Notifications => {
                    close();
                    desktops::windows_panel(b'N' as u16);
                }
                Hit::Nothing => {}
            }
            0
        }
        WM_KEYDOWN => {
            match w as u16 {
                VK_ESCAPE => {
                    close();
                }
                VK_LEFT | VK_PRIOR => change(-1),
                VK_RIGHT | VK_NEXT => change(1),
                VK_HOME => change(0),
                _ => {}
            }
            0
        }
        WM_ACTIVATE if (w & 0xffff) as u32 == WA_INACTIVE => {
            if close() {
                CLOSED_AT.with(|c| c.set(GetTickCount()));
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
