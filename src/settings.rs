use super::*;
pub(super) const CLASS: &str = "OmarchyOverlay.Settings";
#[derive(Clone)]
pub(super) struct Display {
    pub monitor: HMONITOR,
    pub device: String,
    pub work: Rect,
    pub primary: bool,
}
pub(super) unsafe fn displays() -> Vec<Display> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let mut info: MONITORINFOEXW = zeroed();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) != 0 {
            let end = info
                .szDevice
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(info.szDevice.len());
            let r = info.monitorInfo.rcWork;
            (*(data as *mut Vec<Display>)).push(Display {
                monitor,
                device: String::from_utf16_lossy(&info.szDevice[..end]),
                work: Rect {
                    x: r.left,
                    y: r.top,
                    w: r.right - r.left,
                    h: r.bottom - r.top,
                },
                primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            });
        }
        1
    }
    let mut result = vec![];
    EnumDisplayMonitors(
        null_mut(),
        null(),
        Some(collect),
        &mut result as *mut _ as isize,
    );
    result.sort_by(|a: &Display, b| a.device.cmp(&b.device));
    result
}
pub(super) struct Panel {
    pub hwnd: HWND,
    pub monitors: Vec<Display>,
    choices: Vec<String>,
}
unsafe fn combo(parent: HWND, id: usize, items: &[&str], x: i32, y: i32, width: i32) -> HWND {
    let hwnd = child(
        parent,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        id,
        x,
        y,
        width,
        240,
    );
    for item in items {
        SendMessageW(hwnd, CB_ADDSTRING, 0, wide(item).as_ptr() as isize);
    }
    hwnd
}
impl Panel {
    pub(super) unsafe fn new(font: HFONT, brush: HBRUSH) -> Self {
        register_class(CLASS, brush);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            wide(CLASS).as_ptr(),
            wide("Omarchy Overlay — settings and app rules").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            920,
            860,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        let dark: i32 = i32::from(theme::current().dark);
        DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
        child(hwnd, "STATIC", "DESKTOP SETTINGS", 0, 0, 24, 18, 400, 24);
        child(hwnd, "STATIC", "Default", 0, 0, 24, 52, 70, 24);
        combo(
            hwnd,
            60,
            &[
                "Main + stack",
                "Balanced columns",
                "Rows",
                "Grid",
                "Dwindle",
                "Scrolling — Niri-inspired",
            ],
            94,
            48,
            205,
        );
        child(hwnd, "STATIC", "Gap", 0, 0, 320, 52, 42, 24);
        ui_choices::range(hwnd, 61, 365, 48, 64, 0, 80);
        child(hwnd, "STATIC", "Main %", 0, 0, 435, 52, 65, 24);
        ui_choices::range(hwnd, 62, 505, 48, 64, 25, 75);
        combo(hwnd, 63, &["Ctrl + Alt", "Win + Alt", "Alt"], 575, 48, 145);
        child(
            hwnd,
            "BUTTON",
            "Apply",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            64,
            740,
            46,
            125,
            32,
        );
        child(
            hwnd,
            "STATIC",
            "APP RULES — saved per application",
            0,
            0,
            24,
            102,
            840,
            24,
        );
        child(
            hwnd,
            "BUTTON",
            "Choose app / global defaults…",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            76,
            530,
            92,
            335,
            34,
        );
        child(
            hwnd,
            "LISTBOX",
            "",
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
            42,
            24,
            136,
            330,
            306,
        );
        child(hwnd, "STATIC", "Executable name", 0, 0, 380, 136, 180, 24);
        child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            44,
            380,
            162,
            485,
            28,
        );
        child(
            hwnd,
            "STATIC",
            "Title contains (optional)",
            0,
            0,
            380,
            202,
            235,
            24,
        );
        child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            45,
            380,
            228,
            485,
            28,
        );
        child(
            hwnd,
            "STATIC",
            "Window class (optional)",
            0,
            0,
            380,
            268,
            235,
            24,
        );
        child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            46,
            380,
            294,
            485,
            28,
        );
        combo(
            hwnd,
            47,
            &["Tile", "Float", "Ignore completely"],
            380,
            342,
            220,
        );
        combo(
            hwnd,
            48,
            &["Allow opacity", "Keep native opacity"],
            620,
            342,
            245,
        );
        child(
            hwnd,
            "STATIC",
            "Preferred monitor (tiled apps, on adoption)",
            0,
            0,
            380,
            386,
            480,
            24,
        );
        combo(hwnd, 49, &[], 380, 412, 485);
        for (id, text, x, y, width) in [
            (50, "Use selected app", 24, 456, 180),
            (53, "New rule", 216, 456, 138),
            (51, "Save rule", 380, 456, 150),
            (52, "Delete rule", 542, 456, 150),
            (54, "Add work app rules", 704, 456, 161),
        ] {
            child(
                hwnd,
                "BUTTON",
                text,
                BS_OWNERDRAW as u32 | WS_TABSTOP,
                id,
                x,
                y,
                width,
                34,
            );
        }
        child(hwnd, "STATIC", "", SS_PATHELLIPSIS, 65, 24, 512, 840, 24);
        child(hwnd, "STATIC", "", 0, 66, 24, 548, 300, 24);
        child(
            hwnd,
            "BUTTON",
            "Advanced config",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            67,
            700,
            540,
            165,
            32,
        );
        child(hwnd, "STATIC", "Tile = automatic layout. Float = normal placement, with optional opacity. Ignore = no placement or opacity changes.\r\nSpecific title/class rules win over general rules. Legacy exclusions still take priority.\r\nPreferred monitors are not enforced after a manual move; missing monitors keep apps on their current display.\r\nWork starters are editable suggestions. Use selected app to capture the actual executable name.",
            0, 0, 24, 586, 840, 80);
        child(
            hwnd,
            "BUTTON",
            "Profiles / recovery",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            68,
            520,
            540,
            165,
            32,
        );
        child(
            hwnd,
            "BUTTON",
            "Bar / workspaces",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            69,
            340,
            540,
            165,
            32,
        );
        child(
            hwnd,
            "STATIC",
            "Cycle next workspace",
            0,
            0,
            24,
            674,
            220,
            24,
        );
        child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            71,
            250,
            670,
            260,
            28,
        );
        child(
            hwnd,
            "BUTTON",
            "Apply shortcut",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            73,
            524,
            668,
            165,
            32,
        );
        child(
            hwnd,
            "BUTTON",
            "Default",
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            75,
            704,
            668,
            161,
            32,
        );
        child(hwnd, "STATIC", "", SS_PATHELLIPSIS, 74, 24, 710, 840, 24);
        child(
            hwnd,
            "STATIC",
            "Scrolling columns visible",
            0,
            0,
            24,
            750,
            250,
            24,
        );
        ui_choices::range(hwnd, 2340, 280, 746, 90, 1, 4);
        child(
            hwnd,
            "STATIC",
            "Choose Scrolling, then use left/right focus or wheel over running apps.",
            0,
            0,
            24,
            790,
            830,
            24,
        );
        unsafe extern "system" fn set_font(hwnd: HWND, font: LPARAM) -> i32 {
            SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
            1
        }
        EnumChildWindows(hwnd, Some(set_font), font as isize);
        Self {
            hwnd,
            monitors: vec![],
            choices: vec![],
        }
    }
    pub(super) unsafe fn refresh(&mut self, config: &Config, selected: HWND) {
        set_text(GetDlgItem(self.hwnd, 71), &config.cycle_workspace_shortcut);
        self.monitors = displays();
        self.choices = self.monitors.iter().map(|m| m.device.clone()).collect();
        let monitor_combo = GetDlgItem(self.hwnd, 49);
        SendMessageW(monitor_combo, CB_RESETCONTENT, 0, 0);
        SendMessageW(
            monitor_combo,
            CB_ADDSTRING,
            0,
            wide("Leave on current monitor").as_ptr() as isize,
        );
        for monitor in &self.monitors {
            let label = format!(
                "{}{} · {}×{} at {}, {}",
                monitor.device.trim_start_matches("\\\\.\\"),
                if monitor.primary { " (primary)" } else { "" },
                monitor.work.w,
                monitor.work.h,
                monitor.work.x,
                monitor.work.y
            );
            SendMessageW(
                monitor_combo,
                CB_ADDSTRING,
                0,
                wide(&label).as_ptr() as isize,
            );
        }
        for device in config.rules.iter().filter_map(|rule| rule.monitor.as_ref()) {
            if !self.choices.contains(device) {
                self.choices.push(device.clone());
                SendMessageW(
                    monitor_combo,
                    CB_ADDSTRING,
                    0,
                    wide(&format!("{device} (disconnected)")).as_ptr() as isize,
                );
            }
        }
        let list = GetDlgItem(self.hwnd, 42);
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
        for rule in &config.rules {
            SendMessageW(list, LB_ADDSTRING, 0, wide(&rule.label()).as_ptr() as isize);
        }
        let layout = match config.layout.as_str() {
            "columns" => 1,
            "rows" => 2,
            "grid" => 3,
            "dwindle" => 4,
            "scrolling" => 5,
            _ => 0,
        };
        SendMessageW(GetDlgItem(self.hwnd, 60), CB_SETCURSEL, layout, 0);
        ui_choices::select(self.hwnd, 61, &config.gap.to_string());
        ui_choices::select(self.hwnd, 62, &format!("{:.0}", config.main_ratio * 100.0));
        SendMessageW(
            GetDlgItem(self.hwnd, 63),
            CB_SETCURSEL,
            match config.modifiers.as_str() {
                "WinAlt" => 1,
                "Alt" => 2,
                _ => 0,
            },
            0,
        );
        set_text(
            GetDlgItem(self.hwnd, 65),
            &if IsWindow(selected) != 0 {
                format!(
                    "Selected: {} — {}",
                    process_name(selected),
                    text_of(selected)
                )
            } else {
                "Focus a regular app window, then open Settings to select it.".into()
            },
        );
        set_text(
            GetDlgItem(self.hwnd, 66),
            &format!(
                "{} display(s) detected · {} app rule(s) · defaults apply to unmatched apps",
                self.monitors.len(),
                config.rules.len()
            ),
        );
        ui_choices::select(self.hwnd, 2340, &config.scrolling_columns.to_string());
        self.layout_controls();
        self.fill(None);
    }
    pub(super) unsafe fn layout_controls(&self) {
        let scrolling = SendMessageW(GetDlgItem(self.hwnd, 60), CB_GETCURSEL, 0, 0) == 5;
        EnableWindow(GetDlgItem(self.hwnd, 62), i32::from(!scrolling));
        EnableWindow(GetDlgItem(self.hwnd, 2340), i32::from(scrolling));
    }
    pub(super) unsafe fn selected_index(&self) -> Option<usize> {
        let index = SendMessageW(GetDlgItem(self.hwnd, 42), LB_GETCURSEL, 0, 0);
        (index >= 0).then_some(index as usize)
    }
    pub(super) unsafe fn fill(&self, rule: Option<&AppRule>) {
        let default = AppRule::default();
        let rule = rule.unwrap_or(&default);
        for (id, text) in [
            (44, &rule.executable),
            (45, &rule.title_contains),
            (46, &rule.window_class),
        ] {
            set_text(GetDlgItem(self.hwnd, id), text);
        }
        SendMessageW(
            GetDlgItem(self.hwnd, 47),
            CB_SETCURSEL,
            match rule.mode {
                Mode::Tile => 0,
                Mode::Float => 1,
                Mode::Ignore => 2,
            },
            0,
        );
        SendMessageW(
            GetDlgItem(self.hwnd, 48),
            CB_SETCURSEL,
            usize::from(!rule.allow_opacity),
            0,
        );
        let monitor_index = rule
            .monitor
            .as_ref()
            .and_then(|device| self.choices.iter().position(|m| m == device))
            .map_or(0, |i| i + 1);
        SendMessageW(GetDlgItem(self.hwnd, 49), CB_SETCURSEL, monitor_index, 0);
        if let Some(device) = &rule.monitor {
            if !self.monitors.iter().any(|m| m.device == *device) {
                set_text(
                    GetDlgItem(self.hwnd, 66),
                    &format!(
                        "Saved monitor {device} is disconnected. The app stays where it opens."
                    ),
                );
            }
        }
    }
    pub(super) unsafe fn read_rule(&self) -> Result<AppRule, String> {
        let monitor = SendMessageW(GetDlgItem(self.hwnd, 49), CB_GETCURSEL, 0, 0);
        let rule = AppRule {
            executable: text_of(GetDlgItem(self.hwnd, 44)).trim().into(),
            title_contains: text_of(GetDlgItem(self.hwnd, 45)).trim().into(),
            window_class: text_of(GetDlgItem(self.hwnd, 46)).trim().into(),
            mode: match SendMessageW(GetDlgItem(self.hwnd, 47), CB_GETCURSEL, 0, 0) {
                1 => Mode::Float,
                2 => Mode::Ignore,
                _ => Mode::Tile,
            },
            allow_opacity: SendMessageW(GetDlgItem(self.hwnd, 48), CB_GETCURSEL, 0, 0) == 0,
            active_opacity: None,
            inactive_opacity: None,
            coordinate_theme: None,
            backdrop: None,
            monitor: if monitor > 0 {
                self.choices.get(monitor as usize - 1).cloned()
            } else {
                None
            },
        };
        if !rule.valid() {
            return Err("Enter an executable filename such as EXCEL.EXE, without a folder path or wildcards.".into());
        }
        Ok(rule)
    }
    pub(super) unsafe fn general(&self) -> Result<(String, i32, f64, String), String> {
        let gap = text_of(GetDlgItem(self.hwnd, 61))
            .parse::<i32>()
            .ok()
            .filter(|v| (0..=80).contains(v))
            .ok_or("Gap must be 0–80 pixels.")?;
        let ratio = text_of(GetDlgItem(self.hwnd, 62))
            .parse::<u32>()
            .ok()
            .filter(|v| (25..=75).contains(v))
            .ok_or("Main pane must be 25–75 percent.")?;
        let layout = match SendMessageW(GetDlgItem(self.hwnd, 60), CB_GETCURSEL, 0, 0) {
            1 => "columns",
            2 => "rows",
            3 => "grid",
            4 => "dwindle",
            5 => "scrolling",
            _ => "main-stack",
        };
        let modifiers = match SendMessageW(GetDlgItem(self.hwnd, 63), CB_GETCURSEL, 0, 0) {
            1 => "WinAlt",
            2 => "Alt",
            _ => "CtrlAlt",
        };
        Ok((
            layout.into(),
            gap,
            f64::from(ratio) / 100.0,
            modifiers.into(),
        ))
    }
}
