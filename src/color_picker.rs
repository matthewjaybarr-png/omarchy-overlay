//! Omarchy's color picker (Super + Print, hyprpicker): the screen freezes under a
//! crosshair, a label beside the pointer shows the color under it, and a click copies its
//! hex value to the clipboard. Escape or a right-click cancels.
use super::*;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

const CLASS: &str = "OmarchyOverlay.ColorPicker";
/// Posted to the main window with the picked color (0x00RRGGBB) in wParam.
pub(super) const PICKED: u32 = WM_APP + 76;

struct Picker {
    hwnd: HWND,
    owner: HWND,
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    width: i32,
    height: i32,
    point: POINT,
}
thread_local! { static PICKER: RefCell<Option<Picker>> = const { RefCell::new(None) }; }

/// The text the clipboard gets for a COLORREF.
pub fn hex(colorref: u32) -> String {
    format!("#{:02x}{:02x}{:02x}", colorref & 255, (colorref >> 8) & 255, (colorref >> 16) & 255)
}
pub(super) unsafe fn start(owner: HWND) {
    if PICKER.with(|p| p.borrow().is_some()) {
        return;
    }
    static CLASS_DONE: std::sync::Once = std::sync::Once::new();
    CLASS_DONE.call_once(|| {
        let name = wide(CLASS);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_CROSS),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&class);
    });
    let (x, y) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN));
    let (width, height) = (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN));
    let screen = GetDC(null_mut());
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, width, height);
    let old = SelectObject(dc, bitmap as HGDIOBJ);
    BitBlt(dc, 0, 0, width, height, screen, x, y, SRCCOPY | CAPTUREBLT);
    ReleaseDC(null_mut(), screen);
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay color picker").as_ptr(),
        WS_POPUP,
        x,
        y,
        width,
        height,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    if hwnd.is_null() {
        SelectObject(dc, old);
        DeleteObject(bitmap as HGDIOBJ);
        DeleteDC(dc);
        return;
    }
    let mut point: POINT = zeroed();
    GetCursorPos(&mut point);
    PICKER.with(|p| {
        *p.borrow_mut() = Some(Picker {
            hwnd,
            owner,
            dc,
            bitmap,
            old,
            width,
            height,
            point: POINT { x: point.x - x, y: point.y - y },
        })
    });
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
}
unsafe fn close() {
    if let Some(p) = PICKER.with(|p| p.borrow_mut().take()) {
        DestroyWindow(p.hwnd);
        SelectObject(p.dc, p.old);
        DeleteObject(p.bitmap as HGDIOBJ);
        DeleteDC(p.dc);
    }
}
unsafe fn copy(owner: HWND, text: &str) -> bool {
    if OpenClipboard(owner) == 0 {
        return false;
    }
    EmptyClipboard();
    let data: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let handle = GlobalAlloc(GMEM_MOVEABLE, data.len() * 2);
    let mut done = false;
    if !handle.is_null() {
        let target = GlobalLock(handle) as *mut u16;
        if !target.is_null() {
            std::ptr::copy_nonoverlapping(data.as_ptr(), target, data.len());
            GlobalUnlock(handle);
            done = !SetClipboardData(13, handle).is_null();
        }
    }
    CloseClipboard();
    done
}
unsafe fn paint(p: &Picker) {
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(p.hwnd, &mut ps);
    let r = ps.rcPaint;
    BitBlt(dc, r.left, r.top, r.right - r.left, r.bottom - r.top, p.dc, r.left, r.top, SRCCOPY);
    // The label: a swatch of the color under the pointer and its hex value.
    let color = GetPixel(p.dc, p.point.x, p.point.y);
    let palette = theme::current();
    let (lx, ly) = label_origin(p);
    let label = RECT { left: lx, top: ly, right: lx + 132, bottom: ly + 32 };
    SetDCBrushColor(dc, palette.background);
    FillRect(dc, &label, GetStockObject(DC_BRUSH) as HBRUSH);
    SetDCBrushColor(dc, palette.border);
    FrameRect(dc, &label, GetStockObject(DC_BRUSH) as HBRUSH);
    SetDCBrushColor(dc, color);
    FillRect(dc, &RECT { left: lx + 6, top: ly + 6, right: lx + 26, bottom: ly + 26 }, GetStockObject(DC_BRUSH) as HBRUSH);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, palette.text);
    let text = wide(&hex(color));
    let mut tr = RECT { left: lx + 34, top: ly, right: label.right - 6, bottom: label.bottom };
    DrawTextW(dc, text.as_ptr(), (text.len() - 1) as i32, &mut tr, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    EndPaint(p.hwnd, &ps);
}
/// Below and right of the pointer, flipped near the right and bottom edges.
fn label_origin(p: &Picker) -> (i32, i32) {
    let x = if p.point.x + 152 > p.width { p.point.x - 148 } else { p.point.x + 16 };
    let y = if p.point.y + 52 > p.height { p.point.y - 48 } else { p.point.y + 16 };
    (x, y)
}
unsafe fn label_rect(p: &Picker) -> RECT {
    let (x, y) = label_origin(p);
    RECT { left: x - 2, top: y - 2, right: x + 136, bottom: y + 36 }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let point = || POINT { x: (l & 0xffff) as i16 as i32, y: ((l >> 16) & 0xffff) as i16 as i32 };
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            PICKER.with(|p| {
                if let Some(p) = p.borrow().as_ref() {
                    paint(p);
                }
            });
            0
        }
        WM_MOUSEMOVE => {
            PICKER.with(|p| {
                if let Some(p) = p.borrow_mut().as_mut() {
                    let before = label_rect(p);
                    p.point = point();
                    InvalidateRect(hwnd, &before, 0);
                    InvalidateRect(hwnd, &label_rect(p), 0);
                }
            });
            0
        }
        WM_LBUTTONUP => {
            let picked = PICKER.with(|p| {
                p.borrow().as_ref().map(|p| {
                    let at = point();
                    (p.owner, GetPixel(p.dc, at.x, at.y))
                })
            });
            close();
            if let Some((owner, color)) = picked {
                if copy(owner, &hex(color)) {
                    PostMessageW(owner, PICKED, color as usize, 0);
                }
            }
            0
        }
        WM_RBUTTONUP | WM_KILLFOCUS => {
            close();
            0
        }
        WM_KEYDOWN if w == VK_ESCAPE as usize => {
            close();
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}
