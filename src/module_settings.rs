use super::*;
pub(super) const CLASS: &str = "OmarchyOverlay.ModuleSettings";
use crate::bar_model::{BarConfig, DisplayMode, DISPLAY_MODULES};
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — module icons and labels").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        720,
        1040,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    child(
        hwnd,
        "STATIC",
        "MODULE ICONS AND LABELS",
        0,
        0,
        24,
        18,
        660,
        26,
    );
    child(hwnd,"STATIC","Choose each module's appearance. Hover over bar buttons for names,\r\ncurrent status, and click actions. Add/remove modules in Bar / workspaces.",0,0,24,50,660,42);
    for (id, label, x, w) in [
        (454, "All icons", 24, 210),
        (455, "All icons + text", 250, 230),
        (456, "All text", 496, 174),
    ] {
        child(
            hwnd,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            100,
            w,
            34,
        );
    }
    for (i, (_, label)) in DISPLAY_MODULES.iter().enumerate() {
        let y = 148 + i as i32 * 32;
        child(hwnd, "STATIC", label, 0, 0, 24, y + 4, 270, 24);
        let combo = child(
            hwnd,
            "COMBOBOX",
            "",
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
            401 + i,
            310,
            y,
            360,
            180,
        );
        for label in ["Text only", "Icon + text", "Icon only"] {
            SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
    }
    for (id, label, x, width) in [
        (451, "Apply appearance", 24, 210),
        (452, "Restore text defaults", 250, 230),
        (453, "Close", 496, 174),
    ] {
        child(
            hwnd,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            868,
            width,
            36,
        );
    }
    child(hwnd,"STATIC","Applies live and saves to this profile. Running apps use native app icons;\r\nother modules use matching line icons. Workspace icons retain their numbers.",0,0,24,924,660,46);
    unsafe extern "system" fn set_font(hwnd: HWND, font: LPARAM) -> i32 {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        1
    }
    EnumChildWindows(hwnd, Some(set_font), font as isize);
    let dark = i32::from(theme::current().dark);
    DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
    hwnd
}
pub(super) unsafe fn fill(hwnd: HWND, c: &BarConfig) {
    for (i, (module, _)) in DISPLAY_MODULES.iter().enumerate() {
        SendMessageW(
            GetDlgItem(hwnd, 401 + i as i32),
            CB_SETCURSEL,
            match c.display(*module) {
                DisplayMode::Text => 0,
                DisplayMode::IconText => 1,
                DisplayMode::Icon => 2,
            },
            0,
        );
    }
}
pub(super) unsafe fn apply(hwnd: HWND, c: &mut BarConfig) {
    c.module_display.clear();
    for (i, (module, _)) in DISPLAY_MODULES.iter().enumerate() {
        let mode = match SendMessageW(GetDlgItem(hwnd, 401 + i as i32), CB_GETCURSEL, 0, 0) {
            1 => DisplayMode::IconText,
            2 => DisplayMode::Icon,
            _ => DisplayMode::Text,
        };
        c.module_display.insert(*module, mode);
    }
}
