//! The Omarchy Overlay menu: one borderless, rounded list window for the Omarchy-style menu,
//! the app picker, keybinds, open windows and every settings page. Rows are built by
//! App (overlay_pages.rs) and copied in here, so drawing never needs App. Esc always
//! goes back one level and closes the menu from the top level.
use super::*;
use crate::commands::Command;
use std::rc::Rc;

pub(super) const CLASS: &str = "OmarchyOverlay.Overlay";
const WIDTH: i32 = 600;
const ROW: i32 = 30;
const HEADER: i32 = 46;
const FOOTER: i32 = 30;
const VISIBLE: usize = 14;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Level {
    Menu(String),
    Page(&'static str),
    Apps,
    Keybinds,
    Windows,
    WindowPanel(isize),
    Confirm(Box<Command>, String),
    Profile(usize),
    /// An app rule by index; None is the default for all other apps.
    Rule(Option<usize>),
}

pub(super) enum Glyph {
    Char(char),
    Task(Rc<tasks::Identity>),
    Icon(Rc<picker_ui::Icon>),
}

#[derive(Clone)]
pub(super) enum Act {
    None,
    Run(Command),
    Confirm(Command, String),
    Open(Level),
    /// A value chosen on a settings page: page id and value.
    Choose(&'static str, String),
    Launch(usize),
    Window(isize),
    Back,
    Undo,
    Defaults,
}

pub(super) struct Row {
    pub(super) icon: Glyph,
    pub(super) label: String,
    pub(super) detail: String,
    /// Shows a small "Windows" tag: the row opens a Windows screen, not Omarchy Overlay.
    pub(super) windows: bool,
    pub(super) checked: bool,
    pub(super) act: Act,
}
impl Row {
    pub(super) fn new(icon: char, label: impl Into<String>, act: Act) -> Self {
        Self {
            icon: Glyph::Char(icon),
            label: label.into(),
            detail: String::new(),
            windows: false,
            checked: false,
            act,
        }
    }
    pub(super) fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }
    pub(super) fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
    pub(super) fn windows(mut self, windows: bool) -> Self {
        self.windows = windows;
        self
    }
}

pub(super) struct View {
    pub(super) title: String,
    pub(super) rows: Vec<Row>,
    pub(super) hint: String,
    /// App re-ranks the rows itself when the filter changes (the app picker).
    pub(super) ranked: bool,
}

struct Fonts {
    text: HFONT,
    bold: HFONT,
    icon: HFONT,
    small: HFONT,
    dpi: u32,
}
impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            for font in [self.text, self.bold, self.icon, self.small] {
                if !font.is_null() {
                    DeleteObject(font as HGDIOBJ);
                }
            }
        }
    }
}

struct Overlay {
    hwnd: HWND,
    stack: Vec<(Level, usize)>,
    view: View,
    filtered: Vec<usize>,
    selected: usize,
    top: usize,
    filter: String,
    message: String,
    fonts: Option<Fonts>,
    family: String,
    size: i32,
    target: isize,
    /// The monitor the menu opened on (K6): the pointer's for the app picker, the
    /// primary for the menu and settings pages.
    monitor: isize,
    hover: bool,
}
thread_local! { static OVERLAY: RefCell<Option<Overlay>> = const { RefCell::new(None) }; }
thread_local! { static MATERIAL: std::cell::Cell<u32> = const { std::cell::Cell::new(1) }; }

/// The Omarchy Overlay menus material (0.17, 3.1): "solid", "mica" or "acrylic". Separate from
/// the app-windows material so blurring other apps never makes the menu unreadable.
pub(super) fn set_material(material: &str) {
    let value = crate::material::backdrop_type(material);
    if MATERIAL.with(|m| m.replace(value)) != value {
        let h = hwnd();
        if !h.is_null() {
            unsafe {
                style(h);
                InvalidateRect(h, null(), 0);
            }
        }
    }
}
fn material() -> u32 {
    MATERIAL.with(std::cell::Cell::get)
}

fn with<R>(f: impl FnOnce(&mut Overlay) -> R) -> Option<R> {
    OVERLAY.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

pub(super) unsafe fn create() -> HWND {
    let class = WNDCLASSW {
        lpfnWndProc: Some(proc_),
        hInstance: GetModuleHandleW(null()),
        lpszClassName: wide(CLASS).leak().as_ptr(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hIcon: app_icon(),
        style: CS_DROPSHADOW,
        ..zeroed()
    };
    RegisterClassW(&class);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay menu").as_ptr(),
        WS_POPUP,
        0,
        0,
        WIDTH,
        400,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    OVERLAY.with(|cell| {
        *cell.borrow_mut() = Some(Overlay {
            hwnd,
            stack: vec![],
            view: View {
                title: String::new(),
                rows: vec![],
                hint: String::new(),
                ranked: false,
            },
            filtered: vec![],
            selected: 0,
            top: 0,
            filter: String::new(),
            message: String::new(),
            fonts: None,
            family: fonts::ICON_FAMILY.into(),
            size: 12,
            target: 0,
            monitor: 0,
            hover: false,
        })
    });
    hwnd
}

pub(super) unsafe fn destroy() {
    let hwnd = with(|o| o.hwnd);
    OVERLAY.with(|cell| {
        if let Ok(mut overlay) = cell.try_borrow_mut() {
            *overlay = None;
        }
    });
    if let Some(hwnd) = hwnd {
        DestroyWindow(hwnd);
    }
}

pub(super) fn hwnd() -> HWND {
    with(|o| o.hwnd).unwrap_or(null_mut())
}
pub(super) unsafe fn visible() -> bool {
    let h = hwnd();
    !h.is_null() && IsWindowVisible(h) != 0
}
pub(super) fn level() -> Option<Level> {
    with(|o| o.stack.last().map(|(l, _)| l.clone())).flatten()
}
pub(super) fn depth() -> usize {
    with(|o| o.stack.len()).unwrap_or(0)
}
/// The window that was in front when the menu opened.
pub(super) fn target() -> HWND {
    with(|o| o.target as HWND).unwrap_or(null_mut())
}
pub(super) fn filter() -> String {
    with(|o| o.filter.clone()).unwrap_or_default()
}
pub(super) fn set_font(family: &str, size: i32) {
    with(|o| {
        if o.family != family || o.size != size {
            o.family = family.into();
            o.size = size;
            o.fonts = None;
        }
    });
}

fn refilter(o: &mut Overlay) {
    let words: Vec<String> = o
        .filter
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    if o.view.ranked || words.is_empty() {
        o.filtered = (0..o.view.rows.len()).collect();
    } else {
        let mut scored: Vec<(usize, usize)> = o
            .view
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                let text = format!("{} {}", row.label, row.detail).to_lowercase();
                if !words.iter().all(|w| text.contains(w.as_str())) {
                    return None;
                }
                let label = row.label.to_lowercase();
                Some((usize::from(!label.starts_with(words[0].as_str())), i))
            })
            .collect();
        scored.sort();
        o.filtered = scored.into_iter().map(|(_, i)| i).collect();
    }
    o.selected = o.selected.min(o.filtered.len().saturating_sub(1));
    o.top = o.top.min(o.selected);
}

/// Shows a level. `fresh` starts a new menu; otherwise the level is pushed so Esc
/// returns to the one before it.
pub(super) unsafe fn open(level: Level, view: View, target: HWND, fresh: bool) {
    // A menu list starts on its first row, like Omarchy; a page of choices starts on
    // the current choice. (Starting a menu on its first switched-on toggle made the
    // arrow keys land on the wrong row.)
    let menu = matches!(level, Level::Menu(_));
    let monitor = monitor_for(&level) as isize;
    let Some(hwnd) = with(|o| {
        if fresh || o.stack.is_empty() {
            o.stack.clear();
            o.target = target as isize;
            o.monitor = monitor;
        } else if let Some(top) = o.stack.last_mut() {
            top.1 = o.selected;
        }
        o.stack.push((level, 0));
        o.view = view;
        o.filter.clear();
        o.message.clear();
        o.selected = if menu { 0 } else { first_enabled(&o.view) };
        o.top = 0;
        refilter(o);
        o.top = o.selected.saturating_sub(VISIBLE - 1);
        o.hwnd
    }) else {
        return;
    };
    present(hwnd);
}

fn first_enabled(view: &View) -> usize {
    view.rows.iter().position(|r| r.checked).unwrap_or(0)
}

/// Replaces the rows of the level on top, keeping the selection where it was.
pub(super) unsafe fn set_view(view: View) {
    let Some(hwnd) = with(|o| {
        o.view = view;
        refilter(o);
        o.hwnd
    }) else {
        return;
    };
    layout(hwnd);
    style(hwnd);
    InvalidateRect(hwnd, null(), 0);
}

/// Goes back one level. Returns the level now on top, or None when the menu closed.
pub(super) unsafe fn back() -> Option<Level> {
    let (hwnd, next) = with(|o| {
        o.stack.pop();
        (o.hwnd, o.stack.last().cloned())
    })?;
    if next.is_none() {
        ShowWindow(hwnd, SW_HIDE);
    }
    next.map(|(level, _)| level)
}

/// Restores the selection a level had before a deeper level was opened.
pub(super) unsafe fn returned(view: View) {
    let Some(hwnd) = with(|o| {
        let selected = o.stack.last().map_or(0, |(_, s)| *s);
        o.view = view;
        o.filter.clear();
        o.message.clear();
        refilter(o);
        o.selected = selected.min(o.filtered.len().saturating_sub(1));
        o.top = o.selected.saturating_sub(VISIBLE - 1);
        o.hwnd
    }) else {
        return;
    };
    layout(hwnd);
    InvalidateRect(hwnd, null(), 0);
}

pub(super) unsafe fn hide() {
    let h = hwnd();
    if !h.is_null() {
        ShowWindow(h, SW_HIDE);
    }
    with(|o| o.stack.clear());
}

/// A short message in the footer, such as "Saved" or what went wrong.
pub(super) unsafe fn message(text: &str) {
    if let Some(hwnd) = with(|o| {
        o.message = text.replace("\r\n", " ");
        o.hwnd
    }) {
        InvalidateRect(hwnd, null(), 0);
    }
}

/// K6: the app picker (Super+Space, Ctrl+Alt+Space) opens where the mouse is; the
/// Omarchy Overlay menu and every settings page open on the primary monitor, or on the
/// mouse's monitor when there is no primary (it is switched off).
unsafe fn monitor_for(level: &Level) -> HMONITOR {
    let mut pointer: POINT = zeroed();
    GetCursorPos(&mut pointer);
    let at_pointer = MonitorFromPoint(pointer, MONITOR_DEFAULTTONEAREST);
    if *level == Level::Apps {
        return at_pointer;
    }
    // The primary monitor always contains the point (0, 0).
    let primary = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONULL);
    if primary.is_null() {
        at_pointer
    } else {
        primary
    }
}
pub(super) unsafe fn primary_monitor() -> HMONITOR {
    monitor_for(&Level::Menu(String::new()))
}

unsafe fn dpi_for(monitor: HMONITOR) -> u32 {
    let (mut x, mut y) = (96, 96);
    if monitor.is_null() || GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) < 0 {
        return 96;
    }
    x.max(96)
}

/// Sizes the window for its rows on the monitor it opened on, at that monitor's DPI.
unsafe fn layout(hwnd: HWND) {
    let Some((mut monitor, rows)) = with(|o| (o.monitor as HMONITOR, o.filtered.len())) else {
        return;
    };
    if monitor.is_null() {
        monitor = primary_monitor();
    }
    let dpi = dpi_for(monitor);
    with(|o| {
        if o.fonts.as_ref().is_none_or(|f| f.dpi != dpi) {
            // fonts::create falls back to Segoe UI itself when the family is missing.
            let family = o.family.clone();
            o.fonts = Some(Fonts {
                text: fonts::create(&family, o.size, false, dpi),
                bold: fonts::create(&family, o.size, true, dpi),
                icon: fonts::create_icon(o.size + 2, dpi),
                small: fonts::create(&family, (o.size - 3).max(8), false, dpi),
                dpi,
            });
        }
    });
    let unit = |x: i32| x * dpi as i32 / 96;
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(monitor, &mut info);
    let work = info.rcWork;
    let width = unit(WIDTH).min(work.right - work.left - unit(32));
    let shown = rows.clamp(1, VISIBLE) as i32;
    let height = unit(HEADER) + shown * unit(ROW) + unit(FOOTER) + unit(8);
    let x = work.left + (work.right - work.left - width) / 2;
    let y = work.top + ((work.bottom - work.top) / 5).max(unit(24));
    SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE);
}

/// Rounded corners and an accent border from Windows 11; the theme can change while open.
unsafe fn style(hwnd: HWND) {
    let palette = theme::current();
    let round = DWMWCP_ROUND;
    let framed = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
        &round as *const _ as _,
        size_of::<i32>() as u32,
    ) >= 0
        && DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &palette.accent as *const _ as _,
            size_of::<u32>() as u32,
        ) >= 0;
    SetPropW(
        hwnd,
        wide("OmarchyOverlayFramed").as_ptr(),
        framed as usize as HANDLE,
    );
    let dark = i32::from(palette.dark);
    DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
    // Mica/Acrylic behind the whole menu; paint() then leaves the background pixels as
    // a premultiplied tint so the material shows through while text stays opaque.
    let value = material();
    let see_through = value > 1
        && DwmSetWindowAttribute(hwnd, 38, &value as *const _ as _, size_of::<u32>() as u32) >= 0;
    if value <= 1 {
        DwmSetWindowAttribute(hwnd, 38, &value as *const _ as _, size_of::<u32>() as u32);
    }
    let edge = if see_through { -1 } else { 0 };
    let margins = MARGINS {
        cxLeftWidth: edge,
        cxRightWidth: edge,
        cyTopHeight: edge,
        cyBottomHeight: edge,
    };
    let extended = DwmExtendFrameIntoClientArea(hwnd, &margins) >= 0;
    if see_through && extended {
        SetPropW(hwnd, wide("OmarchyOverlayMaterial").as_ptr(), 1usize as HANDLE);
    } else {
        RemovePropW(hwnd, wide("OmarchyOverlayMaterial").as_ptr());
    }
}

unsafe fn present(hwnd: HWND) {
    layout(hwnd);
    style(hwnd);
    crate::desktops::follow(hwnd);
    if IsWindowVisible(hwnd) == 0 {
        let mut animate = 0i32;
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            &mut animate as *mut _ as _,
            0,
        );
        if animate == 0 || AnimateWindow(hwnd, 90, AW_BLEND) == 0 {
            ShowWindow(hwnd, SW_SHOW);
        }
    }
    InvalidateRect(hwnd, null(), 0);
    bring_to_front(hwnd);
}

/// Windows only lets the program that got the last key press take the foreground.
/// A key swallowed by the hook may not count, so the menu joins the input of the
/// window in front for this one call, a documented way to hand focus over.
pub(super) unsafe fn bring_to_front(hwnd: HWND) {
    if SetForegroundWindow(hwnd) != 0 && GetForegroundWindow() == hwnd {
        SetFocus(hwnd);
        return;
    }
    let front = GetForegroundWindow();
    let theirs = GetWindowThreadProcessId(front, null_mut());
    let ours = GetCurrentThreadId();
    if theirs != 0 && theirs != ours && AttachThreadInput(ours, theirs, 1) != 0 {
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        AttachThreadInput(ours, theirs, 0);
    }
    SetFocus(hwnd);
}

fn fill(hdc: HDC, rect: &RECT, color: u32) {
    unsafe {
        SetDCBrushColor(hdc, color);
        FillRect(hdc, rect, GetStockObject(DC_BRUSH) as HBRUSH);
    }
}
unsafe fn text(hdc: HDC, font: HFONT, color: u32, value: &str, rect: &mut RECT, flags: u32) {
    let old = SelectObject(hdc, font as HGDIOBJ);
    SetTextColor(hdc, color);
    let value: Vec<u16> = value.encode_utf16().collect();
    DrawTextW(
        hdc,
        value.as_ptr(),
        value.len() as i32,
        rect,
        flags | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
    );
    SelectObject(hdc, old);
}
fn width_of(hdc: HDC, font: HFONT, value: &str) -> i32 {
    unsafe {
        let old = SelectObject(hdc, font as HGDIOBJ);
        let value: Vec<u16> = value.encode_utf16().collect();
        let mut size: SIZE = zeroed();
        GetTextExtentPoint32W(hdc, value.as_ptr(), value.len() as i32, &mut size);
        SelectObject(hdc, old);
        size.cx
    }
}

unsafe fn paint(hwnd: HWND, target: HDC) {
    let mut client: RECT = zeroed();
    GetClientRect(hwnd, &mut client);
    // Drawn off screen first so the list does not flicker while typing.
    let hdc = CreateCompatibleDC(target);
    // With a material the bitmap is 32-bit top-down so its alpha can be set per pixel.
    let see_through = !GetPropW(hwnd, wide("OmarchyOverlayMaterial").as_ptr()).is_null()
        && client.right > 0
        && client.bottom > 0;
    let mut bits: *mut core::ffi::c_void = null_mut();
    let bitmap = if see_through {
        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = client.right;
        info.bmiHeader.biHeight = -client.bottom;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        CreateDIBSection(hdc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0)
    } else {
        CreateCompatibleBitmap(target, client.right, client.bottom)
    };
    let old_bitmap = SelectObject(hdc, bitmap as HGDIOBJ);
    let framed = !GetPropW(hwnd, wide("OmarchyOverlayFramed").as_ptr()).is_null();
    let mut name = None;
    OVERLAY.with(|cell| {
        let Ok(state) = cell.try_borrow() else {
            return;
        };
        let Some(o) = state.as_ref() else {
            return;
        };
        name = Some(accessible_name(o));
        let Some(f) = o.fonts.as_ref() else {
            return;
        };
        let p = theme::current();
        let unit = |x: i32| x * f.dpi as i32 / 96;
        let icons = fonts::available();
        fill(hdc, &client, p.background);
        SetBkMode(hdc, TRANSPARENT as i32);
        // Header: where you are, then what you typed.
        let mut head = RECT {
            left: unit(18),
            top: 0,
            right: client.right - unit(18),
            bottom: unit(HEADER),
        };
        let crumbs = o.view.title.clone();
        let crumb_width = width_of(hdc, f.bold, &crumbs);
        text(
            hdc,
            f.bold,
            p.accent,
            &crumbs,
            &mut head,
            DT_LEFT | DT_END_ELLIPSIS,
        );
        head.left += crumb_width + unit(14);
        let typed = if o.filter.is_empty() {
            "Type to search".to_owned()
        } else {
            format!("{}▏", o.filter)
        };
        text(
            hdc,
            f.text,
            if o.filter.is_empty() { p.muted } else { p.text },
            &typed,
            &mut head,
            DT_LEFT | DT_END_ELLIPSIS,
        );
        let rule = RECT {
            left: unit(12),
            top: unit(HEADER) - 1,
            right: client.right - unit(12),
            bottom: unit(HEADER),
        };
        fill(hdc, &rule, p.border);
        // Rows.
        let top = unit(HEADER) + unit(4);
        if o.filtered.is_empty() {
            let mut r = RECT {
                left: unit(18),
                top,
                right: client.right - unit(18),
                bottom: top + unit(ROW),
            };
            text(hdc, f.text, p.muted, "Nothing matches", &mut r, DT_LEFT);
        }
        for (slot, &index) in o.filtered.iter().skip(o.top).take(VISIBLE).enumerate() {
            let row = &o.view.rows[index];
            let y = top + slot as i32 * unit(ROW);
            let selected = o.top + slot == o.selected;
            let r = RECT {
                left: unit(6),
                top: y,
                right: client.right - unit(6),
                bottom: y + unit(ROW),
            };
            if selected {
                let region = CreateRoundRectRgn(r.left, r.top, r.right, r.bottom, unit(8), unit(8));
                SetDCBrushColor(hdc, p.surface);
                FillRgn(hdc, region, GetStockObject(DC_BRUSH) as HBRUSH);
                DeleteObject(region as HGDIOBJ);
            }
            let color = if selected { p.accent } else { p.text };
            let mut x = unit(18);
            let icon_box = unit(22);
            match &row.icon {
                Glyph::Char(c) if icons => {
                    let mut ir = RECT {
                        left: x,
                        top: y,
                        right: x + icon_box,
                        bottom: y + unit(ROW),
                    };
                    text(hdc, f.icon, color, &c.to_string(), &mut ir, DT_CENTER);
                }
                Glyph::Task(identity) => {
                    DrawIconEx(
                        hdc,
                        x + unit(3),
                        y + (unit(ROW) - unit(16)) / 2,
                        identity.icon,
                        unit(16),
                        unit(16),
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                }
                Glyph::Icon(icon) => {
                    DrawIconEx(
                        hdc,
                        x + unit(3),
                        y + (unit(ROW) - unit(16)) / 2,
                        icon.0,
                        unit(16),
                        unit(16),
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                }
                _ => {}
            }
            x += icon_box + unit(10);
            let mut right = client.right - unit(18);
            if row.checked {
                let mut cr = RECT {
                    left: right - unit(20),
                    top: y,
                    right,
                    bottom: y + unit(ROW),
                };
                if icons {
                    text(hdc, f.icon, p.accent, "\u{f00c}", &mut cr, DT_RIGHT);
                } else {
                    text(hdc, f.bold, p.accent, "✓", &mut cr, DT_RIGHT);
                }
                right -= unit(26);
            }
            if row.windows {
                let label = "Windows";
                let w = width_of(hdc, f.small, label) + unit(12);
                let tag = RECT {
                    left: right - w,
                    top: y + unit(7),
                    right,
                    bottom: y + unit(ROW) - unit(7),
                };
                let region =
                    CreateRoundRectRgn(tag.left, tag.top, tag.right, tag.bottom, unit(6), unit(6));
                SetDCBrushColor(hdc, p.muted);
                FrameRgn(hdc, region, GetStockObject(DC_BRUSH) as HBRUSH, 1, 1);
                DeleteObject(region as HGDIOBJ);
                let mut tr = tag;
                text(hdc, f.small, p.muted, label, &mut tr, DT_CENTER);
                right -= w + unit(10);
            }
            if !row.detail.is_empty() {
                let w = width_of(hdc, f.small, &row.detail).min((right - x) / 2);
                let mut dr = RECT {
                    left: right - w,
                    top: y,
                    right,
                    bottom: y + unit(ROW),
                };
                text(
                    hdc,
                    f.small,
                    p.muted,
                    &row.detail,
                    &mut dr,
                    DT_RIGHT | DT_END_ELLIPSIS,
                );
                right -= w + unit(12);
            }
            let mut lr = RECT {
                left: x,
                top: y,
                right,
                bottom: y + unit(ROW),
            };
            text(
                hdc,
                if selected { f.bold } else { f.text },
                color,
                &row.label,
                &mut lr,
                DT_LEFT | DT_END_ELLIPSIS,
            );
        }
        // A thin scroll mark when the list is longer than the window.
        if o.filtered.len() > VISIBLE {
            let track = VISIBLE as i32 * unit(ROW);
            let size = (track * VISIBLE as i32 / o.filtered.len() as i32).max(unit(16));
            let offset = (track - size) * o.top as i32 / (o.filtered.len() - VISIBLE) as i32;
            let bar = RECT {
                left: client.right - unit(4),
                top: top + offset,
                right: client.right - unit(2),
                bottom: top + offset + size,
            };
            fill(hdc, &bar, p.border);
        }
        // Footer: the last message, or how to use this level.
        let mut foot = RECT {
            left: unit(18),
            top: client.bottom - unit(FOOTER),
            right: client.right - unit(18),
            bottom: client.bottom - unit(2),
        };
        if o.message.is_empty() {
            text(
                hdc,
                f.small,
                p.muted,
                &o.view.hint,
                &mut foot,
                DT_LEFT | DT_END_ELLIPSIS,
            );
        } else {
            text(
                hdc,
                f.small,
                p.accent,
                &o.message,
                &mut foot,
                DT_LEFT | DT_END_ELLIPSIS,
            );
        }
        if !framed {
            SetDCBrushColor(hdc, p.accent);
            FrameRect(hdc, &client, GetStockObject(DC_BRUSH) as HBRUSH);
        }
    });
    if see_through && !bits.is_null() {
        GdiFlush();
        let p = theme::current();
        let alpha = crate::material::tint_alpha(p.background, &[p.text, p.muted, p.accent]);
        let count = (client.right * client.bottom) as usize;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, count);
        crate::material::apply(pixels, p.background, alpha);
    }
    BitBlt(
        target,
        0,
        0,
        client.right,
        client.bottom,
        hdc,
        0,
        0,
        SRCCOPY,
    );
    SelectObject(hdc, old_bitmap);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(hdc);
    // The menu is drawn by hand, so its window name carries where you are and the
    // highlighted row: Narrator reads it, and the Windows tests check pages by it.
    if let Some(name) = name {
        let mut current = [0u16; 512];
        let len = GetWindowTextW(hwnd, current.as_mut_ptr(), current.len() as i32).max(0) as usize;
        if String::from_utf16_lossy(&current[..len]) != name {
            SetWindowTextW(hwnd, wide(&name).as_ptr());
        }
    }
}

/// "<page title> | <highlighted row> | <its detail>", or just the title when no row
/// matches the filter.
fn accessible_name(o: &Overlay) -> String {
    let row = o.filtered.get(o.selected).and_then(|&i| o.view.rows.get(i));
    let name = match row {
        Some(r) if r.detail.is_empty() => format!("{} | {}", o.view.title, r.label),
        Some(r) => format!("{} | {} | {}", o.view.title, r.label, r.detail),
        None => o.view.title.clone(),
    };
    name.chars().take(500).collect()
}

/// What a key or click asks App to do; the menu itself handles moving around.
fn key(o: &mut Overlay, vk: u16) -> Option<Act> {
    let count = o.filtered.len();
    let page = VISIBLE - 1;
    match vk {
        VK_ESCAPE => return Some(Act::Back),
        VK_LEFT if o.filter.is_empty() && o.stack.len() > 1 => return Some(Act::Back),
        VK_RETURN | VK_RIGHT => {
            if vk == VK_RIGHT && !o.filter.is_empty() {
                return None;
            }
            // An information row does nothing; returning None keeps the selection.
            return o
                .filtered
                .get(o.selected)
                .map(|&i| o.view.rows[i].act.clone())
                .filter(|act| !matches!(act, Act::None));
        }
        VK_DOWN | VK_TAB if count > 0 => o.selected = (o.selected + 1) % count,
        VK_UP if count > 0 => o.selected = (o.selected + count - 1) % count,
        VK_NEXT => o.selected = (o.selected + page).min(count.saturating_sub(1)),
        VK_PRIOR => o.selected = o.selected.saturating_sub(page),
        VK_HOME => o.selected = 0,
        VK_END => o.selected = count.saturating_sub(1),
        VK_BACK => {
            if o.filter.pop().is_none() && o.stack.len() > 1 {
                return Some(Act::Back);
            }
            return Some(Act::None);
        }
        _ => return None,
    }
    if o.selected < o.top {
        o.top = o.selected;
    } else if o.selected >= o.top + VISIBLE {
        o.top = o.selected + 1 - VISIBLE;
    }
    None
}

unsafe fn row_at(hwnd: HWND, l: LPARAM) -> Option<usize> {
    let y = ((l >> 16) & 0xffff) as i16 as i32;
    with(|o| {
        let dpi = o.fonts.as_ref().map_or(96, |f| f.dpi) as i32;
        let top = (HEADER + 4) * dpi / 96;
        let row = ROW * dpi / 96;
        if y < top {
            return None;
        }
        let index = o.top + ((y - top) / row) as usize;
        (index < o.filtered.len() && index < o.top + VISIBLE).then_some(index)
    })
    .flatten()
    .filter(|_| !hwnd.is_null())
}

/// Hands an act to App. App is never borrowed while the menu draws, so this only
/// fails while App itself is opening the menu, and then the key is simply dropped.
unsafe fn dispatch(act: Act) {
    APP.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if let Some(app) = state.as_mut() {
                app.overlay_act(act);
            }
        }
    });
}
/// After the filter changed: the menu filters its own rows; App re-ranks the app picker.
unsafe fn typed(hwnd: HWND) {
    let ranked = with(|o| {
        o.selected = 0;
        o.top = 0;
        o.message.clear();
        refilter(o);
        o.view.ranked
    });
    if ranked == Some(true) {
        refiltered();
    }
    layout(hwnd);
    InvalidateRect(hwnd, null(), 0);
}
unsafe fn refiltered() {
    APP.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if let Some(app) = state.as_mut() {
                app.overlay_filter_changed();
            }
        }
    });
}

unsafe extern "system" fn proc_(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            paint(hwnd, hdc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_PRINTCLIENT => {
            paint(hwnd, w as HDC);
            0
        }
        WM_ERASEBKGND => 1,
        WM_ACTIVATE => {
            // Clicking anywhere else closes the menu, like the Omarchy menu.
            if w & 0xffff == WA_INACTIVE as usize && IsWindowVisible(hwnd) != 0 {
                ShowWindow(hwnd, SW_HIDE);
                with(|o| o.stack.clear());
            }
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let act = with(|o| key(o, w as u16));
            InvalidateRect(hwnd, null(), 0);
            match act {
                Some(Some(Act::None)) => typed(hwnd),
                Some(Some(act)) => dispatch(act),
                _ => {}
            }
            if message == WM_SYSKEYDOWN {
                return DefWindowProcW(hwnd, message, w, l);
            }
            0
        }
        WM_CHAR => {
            // Ctrl+V arrives as character 0x16: paste the clipboard's first line.
            let text = if w == 0x16 {
                clipboard_line()
            } else {
                match char::from_u32(w as u32) {
                    Some(c) if !c.is_control() => c.to_string(),
                    _ => return 0,
                }
            };
            let changed = with(|o| {
                // A search stays short; the text pages (paths, arguments) take a full Windows path.
                let limit = if o.view.ranked { 260 } else { 60 };
                let room = limit - o.filter.chars().count().min(limit);
                if room == 0 || text.is_empty() {
                    return false;
                }
                o.filter.extend(text.chars().take(room));
                true
            })
            .unwrap_or(false);
            if changed {
                typed(hwnd);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some(index) = row_at(hwnd, l) {
                let changed = with(|o| {
                    let changed = o.selected != index;
                    o.selected = index;
                    if !o.hover {
                        o.hover = true;
                        let mut track = TRACKMOUSEEVENT {
                            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                            dwFlags: TME_LEAVE,
                            hwndTrack: hwnd,
                            dwHoverTime: 0,
                        };
                        TrackMouseEvent(&mut track);
                    }
                    changed
                });
                if changed == Some(true) {
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_MOUSELEAVE => {
            with(|o| o.hover = false);
            0
        }
        WM_LBUTTONUP => {
            if let Some(index) = row_at(hwnd, l) {
                let act = with(|o| {
                    o.selected = index;
                    o.view.rows[o.filtered[index]].act.clone()
                });
                if let Some(act) = act {
                    dispatch(act);
                }
            }
            0
        }
        WM_RBUTTONUP => {
            dispatch(Act::Back);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((w >> 16) & 0xffff) as i16;
            with(|o| {
                let max = o.filtered.len().saturating_sub(VISIBLE);
                o.top = if delta > 0 {
                    o.top.saturating_sub(3)
                } else {
                    (o.top + 3).min(max)
                };
                o.selected = o.selected.clamp(
                    o.top,
                    (o.top + VISIBLE - 1).min(o.filtered.len().saturating_sub(1)),
                );
            });
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DPICHANGED => {
            layout(hwnd);
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            with(|o| o.stack.clear());
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

/// The first line of the clipboard text, without control characters; empty when there is none.
unsafe fn clipboard_line() -> String {
    use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    if OpenClipboard(null_mut()) == 0 {
        return String::new();
    }
    let mut text = String::new();
    let handle = GetClipboardData(CF_UNICODETEXT);
    if !handle.is_null() {
        let data = GlobalLock(handle) as *const u16;
        if !data.is_null() {
            let mut len = 0;
            while len < 4096 && *data.add(len) != 0 {
                len += 1;
            }
            text = String::from_utf16_lossy(std::slice::from_raw_parts(data, len));
            GlobalUnlock(handle);
        }
    }
    CloseClipboard();
    text.lines()
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}
