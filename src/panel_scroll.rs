//! Keep the existing fixed control layouts reachable on shorter work areas.
use super::*;

fn dimensions(hwnd: HWND) -> Option<(i32, i32, i32)> {
    match unsafe { window_class(hwnd) }.as_str() {
        appearance_options::CLASS => Some((920, 790, 755)),
        module_settings::CLASS => Some((720, 1040, 1000)),
        app_rules_ui::CLASS => Some((920, 850, 815)),
        appearance_ui::CLASS => Some((900, 800, 780)),
        HELP_CLASS => Some((900, 740, 700)),
        MAIN_CLASS => Some((780, 770, 860)),
        settings::CLASS => Some((920, 860, 830)),
        bar_settings::CLASS => Some((860, 810, 770)),
        _ => None,
    }
}
pub(super) fn supports(hwnd: HWND) -> bool {
    dimensions(hwnd).is_some()
}
unsafe fn info(hwnd: HWND) -> SCROLLINFO {
    let mut info: SCROLLINFO = zeroed();
    info.cbSize = size_of::<SCROLLINFO>() as u32;
    info.fMask = SIF_ALL;
    GetScrollInfo(hwnd, SB_VERT, &mut info);
    info
}
unsafe fn move_children(hwnd: HWND, delta: i32) {
    if delta == 0 {
        return;
    }
    let mut positions = vec![];
    let mut child = GetWindow(hwnd, GW_CHILD);
    while !child.is_null() {
        let mut rect: RECT = zeroed();
        if GetWindowRect(child, &mut rect) != 0 {
            let mut point = POINT {
                x: rect.left,
                y: rect.top,
            };
            ScreenToClient(hwnd, &mut point);
            positions.push((child, point.x, point.y + delta));
        }
        child = GetWindow(child, GW_HWNDNEXT);
    }
    // Move every direct child, including controls currently clipped offscreen.
    // ScrollWindowEx's SW_SCROLLCHILDREN does not handle partially clipped children reliably.
    let flags = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
    let mut deferred = BeginDeferWindowPos(positions.len() as i32);
    for &(child, x, y) in &positions {
        if deferred.is_null() {
            break;
        }
        deferred = DeferWindowPos(deferred, child, null_mut(), x, y, 0, 0, flags);
    }
    if deferred.is_null() || EndDeferWindowPos(deferred) == 0 {
        for (child, x, y) in positions {
            SetWindowPos(child, null_mut(), x, y, 0, 0, flags);
        }
    }
    RedrawWindow(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN,
    );
}
pub(super) unsafe fn update(hwnd: HWND) {
    let Some((_, _, content)) = dimensions(hwnd) else {
        return;
    };
    let old = info(hwnd).nPos;
    let mut rect: RECT = zeroed();
    GetClientRect(hwnd, &mut rect);
    let range = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE,
        nMin: 0,
        nMax: content - 1,
        nPage: (rect.bottom - rect.top).max(1) as u32,
        ..zeroed()
    };
    SetScrollInfo(hwnd, SB_VERT, &range, 1);
    move_children(hwnd, old - info(hwnd).nPos);
}
pub(super) unsafe fn fit(hwnd: HWND) {
    let Some((width, height, _)) = dimensions(hwnd) else {
        return;
    };
    let mut monitor: MONITORINFO = zeroed();
    monitor.cbSize = size_of::<MONITORINFO>() as u32;
    let mut rect: RECT = zeroed();
    // K6: a settings window that is opening (not already on screen) opens centred on
    // the primary monitor, or the mouse's monitor when there is no primary.
    let opening = IsWindowVisible(hwnd) == 0;
    let target = if opening {
        overlay::primary_monitor()
    } else {
        MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
    };
    if GetMonitorInfoW(target, &mut monitor) == 0 || GetWindowRect(hwnd, &mut rect) == 0 {
        return;
    }
    let area = monitor.rcWork;
    if opening {
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
        rect.left = area.left + (area.right - area.left - w) / 2;
        rect.top = area.top + (area.bottom - area.top - h) / 3;
    }
    let width = width.min((area.right - area.left).max(1));
    let height = height.min((area.bottom - area.top).max(1));
    SetWindowPos(
        hwnd,
        null_mut(),
        rect.left.clamp(area.left, area.right - width),
        rect.top.clamp(area.top, area.bottom - height),
        width,
        height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    update(hwnd);
}
pub(super) unsafe fn scroll(hwnd: HWND, command: i32) {
    let current = info(hwnd);
    let next = match command {
        SB_LINEUP => current.nPos - 40,
        SB_LINEDOWN => current.nPos + 40,
        SB_PAGEUP => current.nPos - current.nPage as i32,
        SB_PAGEDOWN => current.nPos + current.nPage as i32,
        SB_THUMBTRACK | SB_THUMBPOSITION => current.nTrackPos,
        SB_TOP => current.nMin,
        SB_BOTTOM => current.nMax,
        _ => return,
    };
    let position = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_POS,
        nPos: next,
        ..zeroed()
    };
    SetScrollInfo(hwnd, SB_VERT, &position, 1);
    move_children(hwnd, current.nPos - info(hwnd).nPos);
}

pub(super) unsafe fn reveal(hwnd: HWND, child: HWND) {
    if !supports(hwnd) {
        return;
    }
    let mut rect: RECT = zeroed();
    let mut client: RECT = zeroed();
    GetWindowRect(child, &mut rect);
    let mut top = POINT {
        x: rect.left,
        y: rect.top,
    };
    ScreenToClient(hwnd, &mut top);
    GetClientRect(hwnd, &mut client);
    let current = info(hwnd);
    let bottom = top.y + rect.bottom - rect.top;
    let delta = if top.y < 0 {
        top.y
    } else {
        (bottom - client.bottom).max(0)
    };
    let position = SCROLLINFO {
        cbSize: size_of::<SCROLLINFO>() as u32,
        fMask: SIF_POS,
        nPos: current.nPos + delta,
        ..zeroed()
    };
    SetScrollInfo(hwnd, SB_VERT, &position, 1);
    move_children(hwnd, current.nPos - info(hwnd).nPos);
}
