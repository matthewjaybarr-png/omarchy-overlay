//! What the bar's popups share (the clock's calendar, the audio panel): a square window
//! in the theme's colors with Omarchy's 2 px border, under a bar module at the top of the
//! screen or over it at the bottom, closed by Escape or a click anywhere else.
use super::*;
use windows_sys::Win32::{
    System::SystemInformation::GetTickCount,
    UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
};

thread_local! {
    /// When a popup last closed because the user clicked elsewhere, and which one: a click
    /// on its own bar module just then closed it, and must not open it again.
    static CLOSED: std::cell::Cell<(u32, usize)> = const { std::cell::Cell::new((0, 0)) };
}

/// Records that popup `id` closed because it lost focus.
pub(super) unsafe fn closed_by_click(id: usize) {
    CLOSED.with(|c| c.set((GetTickCount(), id)));
}
/// Whether popup `id` closed a moment ago because of the click now being handled.
pub(super) unsafe fn just_closed(id: usize) -> bool {
    let (at, which) = CLOSED.with(|c| c.get());
    which == id && GetTickCount().wrapping_sub(at) < 300
}

/// Opens a popup window of `width` × `height` (96-DPI units) by `anchor`, the bar module in
/// screen pixels. Returns the window and its DPI.
pub(super) unsafe fn open(
    class: &str,
    title: &str,
    proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT,
    anchor: RECT,
    width: i32,
    height: i32,
) -> Option<(HWND, u32)> {
    let name = wide(class);
    let mut existing: WNDCLASSW = zeroed();
    if GetClassInfoW(GetModuleHandleW(null()), name.as_ptr(), &mut existing) == 0 {
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&window_class);
    }
    let monitor = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(monitor, &mut info);
    let (mut dpi, mut unused) = (96u32, 96u32);
    GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut unused);
    let dpi = dpi.max(96);
    let unit = |v: i32| v * dpi as i32 / 96;
    let (w, h) = (unit(width), unit(height));
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
        name.as_ptr(),
        wide(title).as_ptr(),
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
        return None;
    }
    let corner = 1u32; // DWMWCP_DONOTROUND: square, as Omarchy's menus
    DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &corner as *const _ as _, size_of::<u32>() as u32);
    Some((hwnd, dpi))
}
/// Shows a popup made by `open` and gives it the keyboard.
pub(super) unsafe fn show(hwnd: HWND) {
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
}

/// `over` mixed into `under` by `amount` (0–1), for COLORREFs.
pub(super) fn blend(over: u32, under: u32, amount: f32) -> u32 {
    let mix = |shift: u32| {
        let a = ((over >> shift) & 255) as f32;
        let b = ((under >> shift) & 255) as f32;
        ((a * amount + b * (1.0 - amount)).round() as u32) << shift
    };
    mix(0) | mix(8) | mix(16)
}

/// A memory DC to paint a popup into without flicker.
pub(super) struct Canvas {
    pub dc: HDC,
    pub rect: RECT,
    pub dpi: u32,
    family: String,
    screen: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
}
impl Canvas {
    /// Starts painting: fills the background and draws Omarchy's 2 px border.
    /// `family` is the text font; empty means the bundled JetBrains Mono.
    pub(super) unsafe fn begin(hwnd: HWND, screen: HDC, dpi: u32, family: &str) -> Self {
        let mut rect: RECT = zeroed();
        GetClientRect(hwnd, &mut rect);
        let dc = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, rect.right.max(1), rect.bottom.max(1));
        let old = SelectObject(dc, bitmap);
        let family = if family.is_empty() { crate::font_names::NL_FAMILY } else { family }.to_string();
        let canvas = Self { dc, rect, dpi, family, screen, bitmap, old };
        let p = theme::current();
        canvas.fill(&rect, p.background);
        let edge = canvas.unit(2).max(1);
        for side in [
            RECT { bottom: edge, ..rect },
            RECT { top: rect.bottom - edge, ..rect },
            RECT { right: edge, ..rect },
            RECT { left: rect.right - edge, ..rect },
        ] {
            canvas.fill(&side, p.border);
        }
        SetBkMode(dc, TRANSPARENT as i32);
        canvas
    }
    pub(super) fn unit(&self, v: i32) -> i32 {
        v * self.dpi as i32 / 96
    }
    pub(super) unsafe fn fill(&self, rect: &RECT, color: u32) {
        SetDCBrushColor(self.dc, color);
        FillRect(self.dc, rect, GetStockObject(DC_BRUSH) as HBRUSH);
    }
    /// Draws one line of text in `rect`, `flags` adding DT_CENTER or DT_RIGHT if wanted.
    pub(super) unsafe fn text(&self, text: &str, rect: RECT, color: u32, bold: bool, size: i32, flags: u32) {
        let font = fonts::create(&self.family, size, bold, self.dpi);
        self.draw(font, text, rect, color, flags);
    }
    /// Draws a Nerd Font icon centred in `rect`.
    pub(super) unsafe fn icon(&self, icon: char, rect: RECT, color: u32, size: i32) {
        if !fonts::available() {
            return;
        }
        let font = fonts::create_icon(size, self.dpi);
        self.draw(font, &icon.to_string(), rect, color, DT_CENTER);
    }
    unsafe fn draw(&self, font: HFONT, text: &str, rect: RECT, color: u32, flags: u32) {
        let old = SelectObject(self.dc, font as HGDIOBJ);
        SetTextColor(self.dc, color);
        let w = wide(text);
        let mut rect = rect;
        DrawTextW(self.dc, w.as_ptr(), (w.len() - 1) as i32, &mut rect, flags | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
        SelectObject(self.dc, old);
        DeleteObject(font as HGDIOBJ);
    }
    /// Copies the finished picture to the window.
    pub(super) unsafe fn finish(self) {
        BitBlt(self.screen, 0, 0, self.rect.right, self.rect.bottom, self.dc, 0, 0, SRCCOPY);
        SelectObject(self.dc, self.old);
        DeleteObject(self.bitmap as HGDIOBJ);
        DeleteDC(self.dc);
    }
}
