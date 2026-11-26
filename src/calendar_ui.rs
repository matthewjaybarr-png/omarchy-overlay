//! The bar clock's calendar, as Omarchy's clock opens one: a month in the theme's colors
//! under the clock, today in the accent color. The mouse wheel or the arrows change the
//! month, a click on the title goes back to today, and "Notifications" opens Windows'
//! own panel. Escape or a click anywhere else closes it.
use super::bar_popup::{self, Canvas};
use super::*;
use crate::{calendar, desktops};
use windows_sys::Win32::{
    Globalization::{GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK, LOCALE_SMONTHNAME1, LOCALE_SSHORTESTDAYNAME1},
    System::SystemInformation::GetLocalTime,
};

const CLASS: &str = "OmarchyOverlay.Calendar";
/// Tells this popup apart from the others in bar_popup.
const ID: usize = 1;
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
thread_local! { static CALENDAR: RefCell<Option<Calendar>> = const { RefCell::new(None) }; }

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

/// Opens the calendar by `anchor` (the clock, in screen pixels), or closes it.
pub(super) unsafe fn toggle(anchor: RECT, family: &str) {
    if close() || bar_popup::just_closed(ID) {
        return;
    }
    let Some((hwnd, dpi)) = bar_popup::open(CLASS, "Omarchy Overlay calendar", proc, anchor, WIDTH, HEIGHT) else {
        return;
    };
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
    bar_popup::show(hwnd);
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

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    CALENDAR.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else { return };
        let p = theme::current();
        let canvas = Canvas::begin(hwnd, screen, c.dpi, &c.family);
        let unit = |v: i32| canvas.unit(v);
        let row = |y: i32, h: i32, x0: i32, x1: i32| RECT { left: x0, top: y, right: x1, bottom: y + unit(h) };
        let left = unit(PAD);
        let grid_w = unit(CELL_W * 7);
        let mut y = unit(PAD);
        // Title row: ‹  November 2026  ›
        let name = c.months.get(c.month as usize - 1).cloned().unwrap_or_default();
        let title = row(y, HEADER - 8, left + unit(CELL_W), left + grid_w - unit(CELL_W));
        canvas.text(&format!("{name} {}", c.year), title, p.text, true, 11, DT_CENTER);
        canvas.text("‹", row(y, HEADER - 8, left, left + unit(CELL_W)), p.accent, true, 13, DT_CENTER);
        canvas.text("›", row(y, HEADER - 8, left + grid_w - unit(CELL_W), left + grid_w), p.accent, true, 13, DT_CENTER);
        y += unit(HEADER);
        let dim = bar_popup::blend(p.text, p.background, 0.55);
        for (column, day) in calendar::weekday_order(c.first_day).into_iter().enumerate() {
            let x = left + unit(CELL_W * column as i32);
            canvas.text(&c.days[day], row(y, CELL_H, x, x + unit(CELL_W)), dim, false, 9, DT_CENTER);
        }
        y += unit(CELL_H);
        for week in calendar::grid(c.year, c.month, c.first_day) {
            for (column, day) in week.into_iter().enumerate() {
                let Some(day) = day else { continue };
                let x = left + unit(CELL_W * column as i32);
                let cell = row(y, CELL_H, x, x + unit(CELL_W));
                let is_today = (c.year, c.month, day) == c.today;
                if is_today {
                    let mark = RECT {
                        left: cell.left + unit(3),
                        right: cell.right - unit(3),
                        top: cell.top + unit(2),
                        bottom: cell.bottom - unit(2),
                    };
                    canvas.fill(&mark, p.accent);
                }
                let color = if is_today { p.background } else { p.text };
                canvas.text(&day.to_string(), cell, color, is_today, 10, DT_CENTER);
            }
            y += unit(CELL_H);
        }
        canvas.fill(&RECT { left, right: left + grid_w, top: y + unit(4), bottom: y + unit(5) }, bar_popup::blend(p.text, p.background, 0.18));
        canvas.text("Notifications", row(y + unit(6), FOOTER - 8, left, left + grid_w), p.accent, false, 10, DT_CENTER);
        canvas.finish();
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
                bar_popup::closed_by_click(ID);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
