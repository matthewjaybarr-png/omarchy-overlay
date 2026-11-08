//! Click-through focus indicator. It never modifies the target window's DWM state.
use super::*;
const CLASS: &str = "OmarchyOverlay.FocusOutline";
pub(super) unsafe fn create(brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay focus indicator").as_ptr(),
        WS_POPUP,
        0,
        0,
        1,
        1,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
    hwnd
}
pub(super) unsafe fn update(app: &App) {
    let target = GetForegroundWindow();
    if !app.config.focus_outline
        || !basic_eligible(target)
        || elevated(target)
        || !can_change_opacity(&app.config, target)
    {
        ShowWindow(app.outline, SW_HIDE);
        return;
    }
    let mut frame: RECT = zeroed();
    if DwmGetWindowAttribute(
        target,
        DWMWA_EXTENDED_FRAME_BOUNDS as u32,
        &mut frame as *mut _ as _,
        size_of::<RECT>() as u32,
    ) < 0
    {
        ShowWindow(app.outline, SW_HIDE);
        return;
    }
    let thickness = (2 * GetDpiForWindow(target).max(96) / 96) as i32;
    let w = frame.right - frame.left + 2 * thickness;
    let h = frame.bottom - frame.top + 2 * thickness;
    let outer = CreateRectRgn(0, 0, w, h);
    let inner = CreateRectRgn(thickness, thickness, w - thickness, h - thickness);
    if outer.is_null() || inner.is_null() {
        if !outer.is_null() {
            DeleteObject(outer);
        }
        if !inner.is_null() {
            DeleteObject(inner);
        }
        ShowWindow(app.outline, SW_HIDE);
        return;
    }
    CombineRgn(outer, outer, inner, RGN_DIFF);
    DeleteObject(inner);
    if SetWindowRgn(app.outline, outer, 1) == 0 {
        DeleteObject(outer);
        return;
    }
    // A private brush keeps the frame in the active palette without changing app rendering.
    SetWindowPos(
        app.outline,
        HWND_TOPMOST,
        frame.left - thickness,
        frame.top - thickness,
        w,
        h,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    let dc = GetDC(app.outline);
    let brush = CreateSolidBrush(theme::current().accent);
    let rect = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    FillRect(dc, &rect, brush);
    DeleteObject(brush);
    ReleaseDC(app.outline, dc);
}
