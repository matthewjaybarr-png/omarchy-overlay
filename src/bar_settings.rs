use super::*;
use crate::bar_model::{BarConfig, Module};
pub(super) const CLASS: &str = "OmarchyOverlay.BarSettings";
unsafe fn edit(hwnd: HWND, id: usize, x: i32, y: i32, w: i32) {
    child(
        hwnd,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        id,
        x,
        y,
        w,
        28,
    );
}
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — bar layout and workspaces").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        860,
        810,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    let dark: i32 = i32::from(theme::current().dark);
    DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
    child(hwnd, "STATIC", "BAR LAYOUT", 0, 0, 24, 18, 700, 24);
    for (id, text, y) in [(201, "Left", 60), (202, "Center", 108), (203, "Right", 156)] {
        child(hwnd, "STATIC", text, 0, 0, 24, y + 4, 80, 24);
        edit(hwnd, id, 112, y, 660);
    }
    child(
        hwnd,
        "BUTTON",
        "Add, remove and arrange bar modules…",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        218,
        24,
        204,
        748,
        36,
    );
    child(
        hwnd,
        "STATIC",
        "Use the visual editor above. The section summaries show the saved layout.",
        0,
        0,
        24,
        246,
        748,
        24,
    );
    for id in [201, 202, 203] {
        SendMessageW(GetDlgItem(hwnd, id), EM_SETREADONLY, 1, 0);
    }
    child(
        hwnd,
        "STATIC",
        "Running app buttons per monitor (0–8)",
        0,
        0,
        24,
        490,
        320,
        24,
    );
    ui_choices::range(hwnd, 210, 360, 486, 80, 0, 8);
    child(
        hwnd,
        "BUTTON",
        "Module icons and labels…",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        214,
        460,
        484,
        312,
        36,
    );
    for (id, label, x, w) in [
        (204, "Height", 24, 60),
        (205, "Font size", 240, 60),
        (206, "Spacing", 460, 60),
    ] {
        child(hwnd, "STATIC", label, 0, 0, x, 272, 85, 24);
        ui_choices::range(
            hwnd,
            id,
            x + 90,
            268,
            w + 22,
            if id == 204 {
                24
            } else if id == 205 {
                10
            } else {
                0
            },
            if id == 204 {
                56
            } else if id == 205 {
                22
            } else {
                24
            },
        );
    }
    child(hwnd, "STATIC", "Font family", 0, 0, 24, 320, 90, 24);
    ui_choices::create(hwnd, 207, 120, 316, 240, &ui_choices::fonts());
    child(hwnd, "STATIC", "Clock", 0, 0, 390, 320, 65, 24);
    let clock = child(
        hwnd,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        208,
        460,
        316,
        312,
        240,
    );
    for (_, label) in crate::bar_model::CLOCK_FORMATS {
        SendMessageW(clock, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
    }
    child(hwnd, "STATIC", "Workspace names", 0, 0, 24, 360, 140, 24);
    ui_choices::create(
        hwnd,
        219,
        180,
        356,
        410,
        &[
            "Custom names".into(),
            "Numbered — four desktops".into(),
            "Work — CAD, Code, Office, Files".into(),
            "Numbered — eight desktops".into(),
        ],
    );
    ui_choices::range(hwnd, 222, 670, 356, 102, 1, 16);
    child(hwnd, "STATIC", "Count", 0, 0, 600, 360, 60, 24);
    ui_choices::create(hwnd, 220, 24, 402, 190, &[]);
    edit(hwnd, 209, 230, 402, 350);
    child(
        hwnd,
        "BUTTON",
        "Set this name",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        221,
        600,
        400,
        172,
        32,
    );
    child(hwnd,"STATIC","Choose a preset or select a desktop and set its name. These labels do not create\r\nWindows desktops. Desktop switching still affects all monitors together.",0,0,24,446,748,42);
    for (id, label, x, w) in [
        (211, "Apply bar settings", 24, 220),
        (212, "Omarchy-style defaults", 260, 240),
        (213, "Windows Task View", 516, 256),
    ] {
        child(
            hwnd,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            540,
            w,
            36,
        );
    }
    child(
        hwnd,
        "BUTTON",
        "Layouts by workspace…",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        215,
        24,
        592,
        260,
        36,
    );
    child(hwnd, "STATIC", "Windows taskbar", 0, 0, 300, 600, 165, 24);
    let mode = child(
        hwnd,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        216,
        470,
        592,
        302,
        220,
    );
    for text in [
        "Use existing preference",
        "Visible",
        "Auto-hide",
        "Keep hidden until shown here",
    ] {
        SendMessageW(mode, CB_ADDSTRING, 0, wide(text).as_ptr() as isize);
    }
    child(hwnd,"STATIC","Changes save to the active profile. Theme and transparency are in Appearance.\r\nIf numbered desktop controls are unavailable, use Windows Task View instead.",0,0,24,640,748,48);
    child(
        hwnd,
        "BUTTON",
        "Show open windows on the bar",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        223,
        24,
        704,
        360,
        38,
    );
    child(
        hwnd,
        "STATIC",
        "Adds Running apps and the drawer; uses each app's native icon.",
        0,
        0,
        402,
        712,
        370,
        44,
    );
    unsafe extern "system" fn set_font(hwnd: HWND, font: LPARAM) -> i32 {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        1
    }
    EnumChildWindows(hwnd, Some(set_font), font as isize);
    hwnd
}
pub(super) unsafe fn fill(hwnd: HWND, c: &BarConfig) {
    for (id, modules) in [(201, &c.left), (202, &c.center), (203, &c.right)] {
        set_text(
            GetDlgItem(hwnd, id),
            &modules
                .iter()
                .map(|m| m.name())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    for (id, text) in [
        (204, c.height.to_string()),
        (205, c.font_size.to_string()),
        (206, c.spacing.to_string()),
        (210, c.task_buttons.to_string()),
        (207, c.font_family.clone()),
    ] {
        ui_choices::select(hwnd, id, &text);
    }
    fill_workspaces(hwnd, &c.workspace_labels);
    SendMessageW(
        GetDlgItem(hwnd, 208),
        CB_SETCURSEL,
        crate::bar_model::CLOCK_FORMATS
            .iter()
            .position(|(name, _)| *name == c.clock_format)
            .unwrap_or(1),
        0,
    );
}
pub(super) unsafe fn read(hwnd: HWND, current: &BarConfig) -> Result<BarConfig, String> {
    let number = |id| {
        text_of(GetDlgItem(hwnd, id))
            .trim()
            .parse::<i32>()
            .map_err(|_| "Height, font size, and spacing must be whole numbers.".to_string())
    };
    let modules = |id| -> Result<Vec<Module>, String> {
        let text = text_of(GetDlgItem(hwnd, id));
        if text.trim().is_empty() {
            return Ok(vec![]);
        }
        text.split(',')
            .map(|name| {
                serde_json::from_value(serde_json::Value::String(name.trim().to_lowercase()))
                    .map_err(|_| format!("Unknown bar module: {}", name.trim()))
            })
            .collect()
    };
    let task_buttons = number(210)?;
    if !(0..=8).contains(&task_buttons) {
        return Err("Choose 0–8 running app buttons per monitor.".into());
    }
    let c = BarConfig {
        module_display: current.module_display.clone(),
        split_task_modules: true,
        task_buttons: task_buttons as usize,
        height: number(204)?,
        font_size: number(205)?,
        spacing: number(206)?,
        font_family: text_of(GetDlgItem(hwnd, 207)).trim().into(),
        clock_format: crate::bar_model::CLOCK_FORMATS
            .get(SendMessageW(GetDlgItem(hwnd, 208), CB_GETCURSEL, 0, 0) as usize)
            .map_or("day-time", |(name, _)| *name)
            .into(),
        workspace_labels: workspace_names(hwnd),
        left: modules(201)?,
        center: modules(202)?,
        right: modules(203)?,
    };
    c.validate()?;
    Ok(c)
}

pub(super) unsafe fn fill_taskbar(h: HWND, c: &Config) {
    SendMessageW(
        GetDlgItem(h, 216),
        CB_SETCURSEL,
        match c.taskbar_mode.as_str() {
            "visible" => 1,
            "auto-hide" => 2,
            "hidden" => 3,
            _ => 0,
        },
        0,
    );
}
pub(super) unsafe fn taskbar_choice(h: HWND) -> &'static str {
    match SendMessageW(GetDlgItem(h, 216), CB_GETCURSEL, 0, 0) {
        1 => "visible",
        2 => "auto-hide",
        3 => "hidden",
        _ => "legacy",
    }
}

pub(super) unsafe fn workspace_names(hwnd: HWND) -> Vec<String> {
    ui_choices::items(hwnd, 220)
}
unsafe fn fill_workspaces(hwnd: HWND, labels: &[String]) {
    let h = GetDlgItem(hwnd, 220);
    SendMessageW(h, CB_RESETCONTENT, 0, 0);
    for name in labels {
        SendMessageW(h, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
    }
    SendMessageW(h, CB_SETCURSEL, 0, 0);
    set_text(
        GetDlgItem(hwnd, 209),
        labels.first().map_or("", String::as_str),
    );
    ui_choices::select(hwnd, 222, &labels.len().to_string());
    SendMessageW(GetDlgItem(hwnd, 219), CB_SETCURSEL, 0, 0);
}
pub(super) unsafe fn choice_command(hwnd: HWND, id: usize, notification: usize) -> bool {
    match id {
        204 | 205 if notification == CBN_SELCHANGE as usize => {
            let height = text_of(GetDlgItem(hwnd, 204)).parse::<i32>().unwrap_or(30);
            let size = text_of(GetDlgItem(hwnd, 205)).parse::<i32>().unwrap_or(13);
            if size > height - 8 {
                if id == 204 {
                    ui_choices::select(hwnd, 205, &(height - 8).clamp(10, 22).to_string());
                } else {
                    ui_choices::select(hwnd, 204, &(size + 8).to_string());
                }
            }
            true
        }
        219 if notification == CBN_SELCHANGE as usize => {
            let labels = match SendMessageW(GetDlgItem(hwnd, 219), CB_GETCURSEL, 0, 0) {
                1 => (1..=4).map(|n| n.to_string()).collect(),
                2 => ["CAD", "Code", "Office", "Files"]
                    .map(String::from)
                    .to_vec(),
                3 => (1..=8).map(|n| n.to_string()).collect(),
                _ => return true,
            };
            let preset = SendMessageW(GetDlgItem(hwnd, 219), CB_GETCURSEL, 0, 0);
            fill_workspaces(hwnd, &labels);
            SendMessageW(GetDlgItem(hwnd, 219), CB_SETCURSEL, preset as usize, 0);
            true
        }
        222 if notification == CBN_SELCHANGE as usize => {
            let count = text_of(GetDlgItem(hwnd, 222))
                .parse::<usize>()
                .unwrap_or(4)
                .clamp(1, 16);
            let mut labels = workspace_names(hwnd);
            labels.resize_with(count, String::new);
            for (i, label) in labels.iter_mut().enumerate() {
                if label.is_empty() {
                    *label = (i + 1).to_string();
                }
            }
            fill_workspaces(hwnd, &labels);
            true
        }
        220 if notification == CBN_SELCHANGE as usize => {
            set_text(GetDlgItem(hwnd, 209), &text_of(GetDlgItem(hwnd, 220)));
            true
        }
        221 => {
            let name = text_of(GetDlgItem(hwnd, 209)).trim().to_string();
            if name.is_empty()
                || name.chars().count() > 12
                || name.chars().any(|c| c.is_control() || c == ',')
            {
                message("Workspace names must contain 1–12 characters without commas.");
                return true;
            }
            let h = GetDlgItem(hwnd, 220);
            let index = SendMessageW(h, CB_GETCURSEL, 0, 0);
            if index >= 0 {
                SendMessageW(h, CB_DELETESTRING, index as usize, 0);
                SendMessageW(
                    h,
                    CB_INSERTSTRING,
                    index as usize,
                    wide(&name).as_ptr() as isize,
                );
                SendMessageW(h, CB_SETCURSEL, index as usize, 0);
                SendMessageW(GetDlgItem(hwnd, 219), CB_SETCURSEL, 0, 0);
            }
            true
        }
        _ => false,
    }
}
