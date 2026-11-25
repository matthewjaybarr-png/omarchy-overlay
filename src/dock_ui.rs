//! The dock window, one per monitor where it shows, after Omarchy's dock: pinned apps
//! (the app picker's favorites) then running apps, with a dot under each running one.
//! It never takes focus; clicks are posted to the main window, which acts on them.
use super::*;
use std::rc::Rc;

pub(super) const DOCK_CLASS: &str = "OmarchyOverlay.Dock";
/// Posted to the main window: wParam 0 left, 1 middle, 2 right click; lParam the icon.
pub(super) const DOCK_MESSAGE: u32 = WM_APP + 75;

#[derive(Clone)]
pub(super) struct View {
    pub label: String,
    pub icon: Rc<picker_ui::Icon>,
    pub running: bool,
    pub focused: bool,
    /// One of its windows flashed for attention and hasn't been in front since.
    pub attention: bool,
}
struct Panel {
    hwnd: HWND,
    monitor: HMONITOR,
    /// Where the dock sits when shown, and the monitor it reveals from.
    rect: RECT,
    screen: RECT,
}
struct State {
    owner: HWND,
    panels: Vec<Panel>,
    views: Vec<View>,
    hover: Option<(isize, usize)>,
    autohide: bool,
    desktop: Option<usize>,
}
thread_local! { static DOCK: RefCell<Option<State>> = const { RefCell::new(None) }; }

unsafe fn register() {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        let name = wide(DOCK_CLASS);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_HAND),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&class);
    });
}
fn scaled(value: i32, dpi: u32) -> i32 {
    value * dpi as i32 / 96
}
unsafe fn monitor_dpi(monitor: HMONITOR) -> u32 {
    use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    let (mut x, mut y) = (96u32, 96u32);
    if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) < 0 {
        return 96;
    }
    x.max(96)
}
/// The icon size in physical pixels on a monitor, for loading pictures.
pub(super) unsafe fn icon_px(monitor: HMONITOR) -> i32 {
    scaled(crate::dock::ICON, monitor_dpi(monitor))
}
/// Shows the dock with `views` on each of `monitors` (monitor, work area, monitor
/// area) and removes it from the others. With `autohide` the docks stay hidden until
/// the pointer reaches the bottom edge of their monitor.
pub(super) unsafe fn update(
    owner: HWND,
    views: Vec<View>,
    monitors: Vec<(HMONITOR, RECT, RECT)>,
    autohide: bool,
    above_taskbar: bool,
) {
    register();
    DOCK.with(|cell| {
        let mut cell = cell.borrow_mut();
        let state = cell.get_or_insert_with(|| State {
            owner,
            panels: vec![],
            views: vec![],
            hover: None,
            autohide,
            desktop: None,
        });
        let changed = state.views.len() != views.len()
            || state.views.iter().zip(&views).any(|(a, b)| {
                a.label != b.label
                    || a.running != b.running
                    || a.focused != b.focused
                    || a.attention != b.attention
                    || !Rc::ptr_eq(&a.icon, &b.icon)
            });
        state.views = views;
        state.autohide = autohide;
        state.panels.retain(|panel| {
            let keep = monitors.iter().any(|(m, _, _)| *m == panel.monitor);
            if !keep {
                DestroyWindow(panel.hwnd);
            }
            keep
        });
        if state.views.is_empty() {
            for panel in state.panels.drain(..) {
                DestroyWindow(panel.hwnd);
            }
            return;
        }
        let desktop = crate::desktops::snapshot().map(|s| s.current);
        let moved_desktop = desktop != state.desktop;
        state.desktop = desktop;
        for (monitor, work, screen) in monitors {
            let dpi = monitor_dpi(monitor);
            let (w, h) = crate::dock::size(state.views.len());
            let (w, h) = (scaled(w, dpi), scaled(h, dpi));
            // 48 px is Windows 11's taskbar height.
            let margin = scaled(crate::dock::MARGIN + if above_taskbar { 48 } else { 0 }, dpi);
            let rect = RECT {
                left: work.left + (work.right - work.left - w) / 2,
                top: work.bottom - h - margin,
                right: work.left + (work.right - work.left - w) / 2 + w,
                bottom: work.bottom - margin,
            };
            let index = match state.panels.iter().position(|p| p.monitor == monitor) {
                Some(index) => index,
                None => {
                    let hwnd = CreateWindowExW(
                        WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                        wide(DOCK_CLASS).as_ptr(),
                        wide("Omarchy Overlay dock").as_ptr(),
                        WS_POPUP,
                        rect.left,
                        rect.top,
                        w,
                        h,
                        null_mut(),
                        null_mut(),
                        GetModuleHandleW(null()),
                        null(),
                    );
                    if hwnd.is_null() {
                        continue;
                    }
                    let corner = 2u32; // DWMWCP_ROUND
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                        &corner as *const _ as _,
                        size_of::<u32>() as u32,
                    );
                    state.panels.push(Panel {
                        hwnd,
                        monitor,
                        rect,
                        screen,
                    });
                    crate::desktops::follow(hwnd);
                    state.panels.len() - 1
                }
            };
            let panel = &mut state.panels[index];
            let resized = panel.rect.left != rect.left
                || panel.rect.top != rect.top
                || panel.rect.right != rect.right
                || panel.rect.bottom != rect.bottom;
            panel.rect = rect;
            panel.screen = screen;
            if moved_desktop {
                crate::desktops::follow(panel.hwnd);
            }
            let show = !autohide || IsWindowVisible(panel.hwnd) != 0;
            if resized || IsWindowVisible(panel.hwnd) != i32::from(show) {
                SetWindowPos(
                    panel.hwnd,
                    HWND_TOPMOST,
                    rect.left,
                    rect.top,
                    w,
                    h,
                    SWP_NOACTIVATE | if show { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW },
                );
            }
            if changed || resized {
                InvalidateRect(panel.hwnd, null(), 0);
            }
        }
    });
}
/// For the auto-hiding dock: shows a dock while the pointer is at the bottom edge of
/// its monitor or over it, and hides it again when the pointer leaves.
pub(super) unsafe fn reveal_tick() {
    DOCK.with(|cell| {
        let Ok(cell) = cell.try_borrow() else {
            return;
        };
        let Some(state) = cell.as_ref() else {
            return;
        };
        if !state.autohide {
            return;
        }
        let mut point: POINT = zeroed();
        if GetCursorPos(&mut point) == 0 {
            return;
        }
        for panel in &state.panels {
            let s = panel.screen;
            let at_edge = point.x >= s.left && point.x < s.right && point.y >= s.bottom - 2 && point.y < s.bottom;
            let r = panel.rect;
            let over = point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom;
            let shown = IsWindowVisible(panel.hwnd) != 0;
            if !shown && at_edge {
                ShowWindow(panel.hwnd, SW_SHOWNOACTIVATE);
            } else if shown && !at_edge && !over {
                ShowWindow(panel.hwnd, SW_HIDE);
            }
        }
    });
}
pub(super) unsafe fn stop() {
    DOCK.with(|cell| {
        if let Some(state) = cell.borrow_mut().take() {
            for panel in state.panels {
                DestroyWindow(panel.hwnd);
            }
        }
    });
}
/// Repaints every dock, after a theme change.
pub(super) unsafe fn redraw() {
    DOCK.with(|cell| {
        if let Ok(cell) = cell.try_borrow() {
            for panel in cell.iter().flat_map(|s| &s.panels) {
                InvalidateRect(panel.hwnd, null(), 0);
            }
        }
    });
}
unsafe fn paint(state: &State, hwnd: HWND) {
    let palette = theme::current();
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    let mut bounds: RECT = zeroed();
    GetClientRect(hwnd, &mut bounds);
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, bounds.right.max(1), bounds.bottom.max(1));
    let old = SelectObject(dc, bitmap);
    SetDCBrushColor(dc, palette.surface);
    FillRect(dc, &bounds, GetStockObject(DC_BRUSH) as HBRUSH);
    SetDCBrushColor(dc, palette.border);
    FrameRect(dc, &bounds, GetStockObject(DC_BRUSH) as HBRUSH);
    let dpi = GetDpiForWindow(hwnd).max(96);
    let unit = |v: i32| scaled(v, dpi);
    let (pad, slot, icon) = (unit(crate::dock::PAD), unit(crate::dock::SLOT), unit(crate::dock::ICON));
    for (index, view) in state.views.iter().enumerate() {
        let left = pad + slot * index as i32;
        let cell = RECT {
            left: left + unit(2),
            top: pad + unit(2),
            right: left + slot - unit(2),
            bottom: pad + slot - unit(2),
        };
        if state.hover == Some((hwnd as isize, index)) {
            SetDCBrushColor(dc, crate::material::selection(palette.border, palette.background, palette.accent));
            FillRect(dc, &cell, GetStockObject(DC_BRUSH) as HBRUSH);
        }
        if view.attention {
            // The taskbar's flashing button: the accent around the app that wants you.
            SetDCBrushColor(dc, palette.accent);
            for inset in 0..unit(2) {
                let ring = RECT {
                    left: cell.left + inset,
                    top: cell.top + inset,
                    right: cell.right - inset,
                    bottom: cell.bottom - inset,
                };
                FrameRect(dc, &ring, GetStockObject(DC_BRUSH) as HBRUSH);
            }
        }
        DrawIconEx(
            dc,
            left + (slot - icon) / 2,
            pad + (slot - icon) / 2 - unit(3),
            view.icon.0,
            icon,
            icon,
            0,
            null_mut(),
            DI_NORMAL,
        );
        if view.running {
            let width = if view.focused { unit(14) } else { unit(5) };
            let mark = RECT {
                left: left + (slot - width) / 2,
                top: pad + slot - unit(6),
                right: left + (slot + width) / 2,
                bottom: pad + slot - unit(3),
            };
            SetDCBrushColor(dc, if view.focused { palette.accent } else { palette.muted });
            FillRect(dc, &mark, GetStockObject(DC_BRUSH) as HBRUSH);
        }
    }
    BitBlt(screen, 0, 0, bounds.right, bounds.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old);
    DeleteObject(bitmap);
    DeleteDC(dc);
    EndPaint(hwnd, &ps);
}
/// The icon under a client point, in 96-DPI dock coordinates.
unsafe fn hit(hwnd: HWND, count: usize, l: LPARAM) -> Option<usize> {
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let x = (l & 0xffff) as i16 as i32 * 96 / dpi;
    let y = ((l >> 16) & 0xffff) as i16 as i32 * 96 / dpi;
    crate::dock::hit(x, y, count)
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE as isize;
    }
    if msg == WM_ERASEBKGND {
        return 1;
    }
    let handled = DOCK.with(|cell| {
        let Ok(mut cell) = cell.try_borrow_mut() else {
            return None;
        };
        let state = cell.as_mut()?;
        match msg {
            WM_PAINT => {
                paint(state, hwnd);
                Some(0)
            }
            WM_MOUSEMOVE => {
                let index = hit(hwnd, state.views.len(), l);
                let hover = index.map(|i| (hwnd as isize, i));
                if hover != state.hover {
                    state.hover = hover;
                    InvalidateRect(hwnd, null(), 0);
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut track);
                }
                Some(0)
            }
            WM_MOUSELEAVE => {
                state.hover = None;
                InvalidateRect(hwnd, null(), 0);
                Some(0)
            }
            WM_LBUTTONUP | WM_MBUTTONUP | WM_RBUTTONUP => {
                if let Some(index) = hit(hwnd, state.views.len(), l) {
                    let kind = match msg {
                        WM_LBUTTONUP => 0,
                        WM_MBUTTONUP => 1,
                        _ => 2,
                    };
                    PostMessageW(state.owner, DOCK_MESSAGE, kind, index as isize);
                }
                Some(0)
            }
            _ => None,
        }
    });
    handled.unwrap_or_else(|| DefWindowProcW(hwnd, msg, w, l))
}
