use super::*;
use windows_sys::Win32::UI::Controls::Dialogs::*;
pub(super) const CLASS: &str = "OmarchyOverlay.Profiles";
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — profiles and recovery").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        780,
        490,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    let dark: i32 = i32::from(theme::current().dark);
    DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
    child(hwnd, "STATIC", "", SS_PATHELLIPSIS, 90, 24, 20, 715, 50);
    child(hwnd,"STATIC","Shared profile: layout, themes, shortcuts, and app rules.\r\nPer PC: monitor assignments, window handles, logs, and recovery records.",0,0,24,80,715,50);
    for (id, text, x, y, w) in [
        (80, "Make this folder portable", 24, 148, 225),
        (81, "Export profile…", 265, 148, 225),
        (82, "Import profile…", 506, 148, 225),
        (83, "Reload saved preferences", 24, 200, 225),
        (84, "Reset preferences…", 265, 200, 225),
        (85, "Open recovery folder", 506, 200, 225),
        (86, "Restore desktop and exit", 24, 252, 225),
        (87, "View last recovery report", 265, 252, 225),
    ] {
        child(
            hwnd,
            "BUTTON",
            text,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            y,
            w,
            36,
        );
    }
    child(hwnd,"STATIC","Import and reset keep a last backup in the local recovery folder.\r\nRecovery uses a background mode of this same executable, without admin or a service.\r\nAfter an unexpected exit, the guard restores surviving windows and the Windows taskbar.\r\nClose Omarchy Overlay before ejecting or moving its folder; keep the profile beside the executable.",0,0,24,316,715,90);
    unsafe extern "system" fn set_font(hwnd: HWND, font: LPARAM) -> i32 {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        1
    }
    EnumChildWindows(hwnd, Some(set_font), font as isize);
    hwnd
}
pub(super) unsafe fn choose(owner: HWND, save: bool) -> Result<Option<PathBuf>, String> {
    let mut buffer = [0u16; 32768];
    if save {
        for (i, c) in wide(profile::FILE_NAME).iter().enumerate() {
            buffer[i] = *c;
        }
    }
    let filter = wide("Omarchy Overlay profile (*.json)\0*.json\0All files\0*.*\0");
    let title = wide(if save {
        "Export Omarchy Overlay profile"
    } else {
        "Import Omarchy Overlay profile"
    });
    let extension = wide("json");
    let mut info: OPENFILENAMEW = zeroed();
    info.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    info.hwndOwner = owner;
    info.lpstrFilter = filter.as_ptr();
    info.lpstrFile = buffer.as_mut_ptr();
    info.nMaxFile = buffer.len() as u32;
    info.lpstrTitle = title.as_ptr();
    info.lpstrDefExt = extension.as_ptr();
    info.Flags = OFN_EXPLORER
        | OFN_NOCHANGEDIR
        | OFN_PATHMUSTEXIST
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };
    let ok = if save {
        GetSaveFileNameW(&mut info)
    } else {
        GetOpenFileNameW(&mut info)
    };
    if ok == 0 {
        let error = CommDlgExtendedError();
        if error != 0 {
            return Err(format!("Windows file dialog failed: {error}"));
        }
        return Ok(None);
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Ok(Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..end],
    ))))
}
