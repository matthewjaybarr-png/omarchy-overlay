//! Choose actual running windows and edit global or app-specific policy.
use super::*;
pub(super) const CLASS: &str = "OmarchyOverlay.AppRules";
fn mode_index(mode: Mode) -> usize {
    match mode {
        Mode::Tile => 0,
        Mode::Float => 1,
        Mode::Ignore => 2,
    }
}
unsafe fn choice(hwnd: HWND, id: i32) -> usize {
    SendMessageW(GetDlgItem(hwnd, id), CB_GETCURSEL, 0, 0).max(0) as usize
}
unsafe fn combo(hwnd: HWND, id: usize, items: &[&str], x: i32, y: i32, w: i32) {
    let c = child(
        hwnd,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        id,
        x,
        y,
        w,
        240,
    );
    for text in items {
        SendMessageW(c, CB_ADDSTRING, 0, wide(text).as_ptr() as isize);
    }
}
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let h = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — apps and global defaults").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        920,
        850,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    child(
        h,
        "STATIC",
        "APPS AND GLOBAL DEFAULTS",
        0,
        0,
        24,
        20,
        860,
        26,
    );
    combo(
        h,
        1000,
        &["Selected running app", "Apps without an explicit rule"],
        24,
        64,
        840,
    );
    child(
        h,
        "LISTBOX",
        "",
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
        1001,
        24,
        108,
        420,
        370,
    );
    child(h, "STATIC", "", 0, 1002, 470, 108, 395, 100);
    child(h, "STATIC", "Window behavior", 0, 0, 470, 214, 395, 24);
    combo(
        h,
        1005,
        &["Tile automatically", "Float normally", "Ignore completely"],
        470,
        242,
        395,
    );
    child(h, "STATIC", "Opacity", 0, 0, 470, 280, 395, 24);
    combo(
        h,
        1006,
        &["Allow opacity controls", "Keep native opacity"],
        470,
        308,
        395,
    );
    for (id, label, x) in [(1007, "Active %", 470), (1008, "Inactive %", 675)] {
        child(h, "STATIC", label, 0, 0, x, 350, 185, 24);
        child(
            h,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            id,
            x,
            378,
            185,
            28,
        );
    }
    child(h, "STATIC", "Window frame theme", 0, 0, 470, 420, 395, 24);
    combo(
        h,
        1009,
        &[
            "Use global choice",
            "Keep native frame",
            "Coordinate with Omarchy Overlay",
        ],
        470,
        448,
        395,
    );
    child(
        h,
        "STATIC",
        "Frame material (supported surfaces)",
        0,
        0,
        470,
        486,
        395,
        24,
    );
    combo(
        h,
        1012,
        &[
            "Use global material",
            "Keep native material",
            "Mica",
            "Acrylic blur",
        ],
        470,
        514,
        395,
    );
    for (id, label, x, w) in [
        (1004, "Refresh running apps", 24, 200),
        (1003, "Advanced rule filters…", 240, 204),
        (1010, "Save and apply", 470, 395),
    ] {
        child(
            h,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            570,
            w,
            36,
        );
    }
    child(h, "STATIC", "", 0, 1011, 24, 626, 840, 90);
    child(h,"STATIC","Blank app opacity values use the global percentages. Windows keep their own content rendering.\r\nDialogs, elevated apps and explicit exclusions remain protected. Global defaults do not replace app rules.",0,0,24,730,840,50);
    unsafe extern "system" fn font_child(h: HWND, f: LPARAM) -> i32 {
        SendMessageW(h, WM_SETFONT, f as usize, 1);
        1
    }
    EnumChildWindows(h, Some(font_child), font as isize);
    h
}
unsafe fn target(app: &App) -> Option<tasks::Task> {
    let i = SendMessageW(GetDlgItem(app.rule_center, 1001), LB_GETCURSEL, 0, 0);
    if i < 0 {
        return None;
    }
    app.rule_windows
        .get(i as usize)
        .cloned()
        .filter(|t| t.valid())
}
unsafe fn matching_index(app: &App, hwnd: HWND) -> Option<usize> {
    let r = rule_for(&app.config, hwnd)?;
    app.config
        .rules
        .iter()
        .position(|other| std::ptr::eq(r, other))
}
unsafe fn fill(app: &App) {
    let h = app.rule_center;
    let global = choice(h, 1000) == 1;
    // 0.20: the running-apps list stays enabled in global mode. Disabled, Windows drew it in
    // its own grey (2.8:1 on the navy surface); choosing an app there switches back to it.
    EnableWindow(GetDlgItem(h, 1003), i32::from(!global));
    // 0.21: a highlighted app in global mode looked like the app being edited.
    if global {
        SendMessageW(GetDlgItem(h, 1001), LB_SETCURSEL, usize::MAX, 0);
    }
    let task = target(app);
    let rule = task
        .as_ref()
        .and_then(|t| matching_index(app, t.hwnd))
        .map(|i| &app.config.rules[i]);
    let policy = if global {
        app.config.policy("", "", "")
    } else if let Some(t) = &task {
        window_policy(&app.config, t.hwnd)
    } else {
        app.config.policy("", "", "")
    };
    let description = if global {
        "Mode and opacity permission apply to unmatched apps.\r\nBlank appearance values in app rules inherit these defaults.\r\nPick an app in the list to edit its own rule instead.".into()
    } else if let Some(t) = &task {
        format!(
            "{} · {}\r\nEffective: {} · {}\r\n{}",
            rules::app_name(&process_name(t.hwnd)),
            process_name(t.hwnd),
            policy.mode.label(),
            if rule.is_some() {
                "app rule"
            } else {
                "global default"
            },
            app.window_behavior_reason(t.hwnd)
        )
    } else {
        "Select an actual running app on the left.\r\nNew and classic Outlook are separate applications.".into()
    };
    set_text(GetDlgItem(h, 1002), &description);
    for (id, index) in [
        (1005, mode_index(policy.mode)),
        (1006, usize::from(!policy.allow_opacity)),
    ] {
        SendMessageW(GetDlgItem(h, id), CB_SETCURSEL, index, 0);
    }
    for (id, value) in [
        (
            1007,
            if global {
                Some(app.config.active_opacity)
            } else {
                rule.and_then(|r| r.active_opacity)
            },
        ),
        (
            1008,
            if global {
                Some(app.config.inactive_opacity)
            } else {
                rule.and_then(|r| r.inactive_opacity)
            },
        ),
    ] {
        set_text(
            GetDlgItem(h, id),
            &value.map(|n| n.to_string()).unwrap_or_default(),
        );
    }
    let frame = if global {
        Some(app.config.coordinate_windows)
    } else {
        rule.and_then(|r| r.coordinate_theme)
    };
    SendMessageW(
        GetDlgItem(h, 1009),
        CB_SETCURSEL,
        match frame {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        },
        0,
    );
    let material = if global {
        Some(app.config.window_backdrop.as_str())
    } else {
        rule.and_then(|r| r.backdrop.as_deref())
    };
    SendMessageW(
        GetDlgItem(h, 1012),
        CB_SETCURSEL,
        match material {
            None => 0,
            Some("mica") => 2,
            Some("acrylic") => 3,
            _ => 1,
        },
        0,
    );
}
pub(super) unsafe fn command(app: &mut App, id: usize, notification: usize) {
    let h = app.rule_center;
    if id == 1004 {
        app.show_app_rules();
        return;
    }
    if id == 1001 && notification == LBN_SELCHANGE as usize && choice(h, 1000) == 1 {
        SendMessageW(GetDlgItem(h, 1000), CB_SETCURSEL, 0, 0);
    }
    if id == 1000 && notification == CBN_SELCHANGE as usize
        || id == 1001 && notification == LBN_SELCHANGE as usize
    {
        fill(app);
        return;
    }
    if id == 1003 {
        if let Some(task) = target(app) {
            app.selected = task.hwnd;
            app.show_settings();
            app.edit_window_rule(task.hwnd);
            ShowWindow(h, SW_HIDE);
        }
        return;
    }
    if id != 1010 {
        return;
    }
    let result = (|| -> Result<String, String> {
        let global = choice(h, 1000) == 1;
        let number = |id, min| -> Result<Option<u8>, String> {
            let text = text_of(GetDlgItem(h, id));
            if text.trim().is_empty() && !global {
                return Ok(None);
            }
            let v = text.trim().parse::<u8>().map_err(|_| {
                "Enter a percentage, or leave an app value blank to use the global value."
            })?;
            if !(min..=100).contains(&v) {
                return Err(format!("Opacity must be {min}–100%."));
            }
            Ok(Some(v))
        };
        let active = number(1007, 50)?;
        let inactive = number(1008, 70)?;
        let mode = [Mode::Tile, Mode::Float, Mode::Ignore][choice(h, 1005).min(2)];
        let allow = choice(h, 1006) == 0;
        let material = match choice(h, 1012) {
            1 => Some("native".to_owned()),
            2 => Some("mica".to_owned()),
            3 => Some("acrylic".to_owned()),
            _ => None,
        };
        let frame = match choice(h, 1009) {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        };
        if global {
            app.config.default_mode = mode;
            app.config.default_allow_opacity = allow;
            app.config.active_opacity = active.unwrap_or(100);
            app.config.inactive_opacity = inactive.unwrap_or(100);
            app.config.coordinate_windows = frame.unwrap_or(false);
            if let Some(material) = material {
                app.config.window_backdrop = if material == "native" {
                    "solid".into()
                } else {
                    material
                };
                app.apply_own_appearance();
            }
            app.apply_rules();
            Ok(format!(
                "Saved global {} defaults. Explicit app rules are unchanged.",
                mode.label()
            ))
        } else {
            let task = target(app)
                .ok_or("That window is no longer available. Refresh and choose an app again.")?;
            let index = matching_index(app, task.hwnd);
            let mut rule = index
                .map(|i| app.config.rules[i].clone())
                .unwrap_or(AppRule {
                    executable: process_name(task.hwnd),
                    ..Default::default()
                });
            rule.mode = mode;
            rule.allow_opacity = allow;
            rule.active_opacity = active;
            rule.inactive_opacity = inactive;
            rule.coordinate_theme = frame;
            rule.backdrop = material;
            if !rule.valid() {
                return Err("The app rule is invalid.".into());
            }
            app.selected = task.hwnd;
            if crate::config::always_ignored(&rule.executable) {
                app.save_app_rule(index, rule.clone());
                return Ok(
                    "Mastercam is always left alone: its rule stays Ignore (always).".into(),
                );
            }
            let count = app.save_app_rule(index, rule.clone());
            Ok(format!("Saved {} for {} ({}). Applied to {count} matching window(s). Other apps keep their choices.",mode.label(),rules::app_name(&rule.executable),rule.executable))
        }
    })();
    fill(app);
    set_text(
        GetDlgItem(h, 1011),
        &match result {
            Ok(s) => s,
            Err(e) => e,
        },
    );
}
impl App {
    pub(super) unsafe fn show_app_rules(&mut self) {
        let selected = self.selected;
        self.rule_windows = tasks::collect();
        let list = GetDlgItem(self.rule_center, 1001);
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
        for task in &self.rule_windows {
            let exe = process_name(task.hwnd);
            let label = format!("{} · {} — {}", rules::app_name(&exe), exe, task.title);
            SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        let index = self
            .rule_windows
            .iter()
            .position(|t| t.hwnd == selected)
            .unwrap_or(0);
        SendMessageW(list, LB_SETCURSEL, index, 0);
        SendMessageW(GetDlgItem(self.rule_center, 1000), CB_SETCURSEL, 0, 0);
        fill(self);
        set_text(GetDlgItem(self.rule_center,1011),"Choose the actual app you want to change, or choose global defaults above. Save and apply changes only that choice.");
        crate::desktops::follow(self.rule_center);
        panel_scroll::fit(self.rule_center);
        ShowWindow(self.rule_center, SW_SHOWNORMAL);
        SetForegroundWindow(self.rule_center);
    }
    pub(super) unsafe fn edit_window_rule(&mut self, hwnd: HWND) {
        let index = matching_index(self, hwnd);
        SendMessageW(
            GetDlgItem(self.settings.hwnd, 42),
            LB_SETCURSEL,
            index.unwrap_or(usize::MAX),
            0,
        );
        let rule = index
            .map(|i| self.config.rules[i].clone())
            .unwrap_or(AppRule {
                executable: process_name(hwnd),
                ..Default::default()
            });
        self.settings.fill(Some(&rule));
    }
    pub(super) unsafe fn save_app_rule(
        &mut self,
        index: Option<usize>,
        mut rule: AppRule,
    ) -> usize {
        if crate::config::always_ignored(&rule.executable) {
            // Mastercam is a fixed Ignore: whatever was asked, only Ignore is stored.
            rule = AppRule {
                executable: rule.executable,
                title_contains: rule.title_contains,
                window_class: rule.window_class,
                mode: Mode::Ignore,
                allow_opacity: false,
                ..AppRule::default()
            };
        }
        let mut affected = vec![];
        unsafe extern "system" fn collect(h: HWND, p: LPARAM) -> i32 {
            (*(p as *mut Vec<HWND>)).push(h);
            1
        }
        EnumWindows(Some(collect), &mut affected as *mut _ as isize);
        affected.retain(|&h| {
            rules::matching(
                std::slice::from_ref(&rule),
                &process_name(h),
                &text_of(h),
                &window_class(h),
            )
            .is_some()
        });
        let saved_index = if let Some(i) = index.filter(|i| *i < self.config.rules.len()) {
            self.config.rules[i] = rule.clone();
            i
        } else {
            self.config.rules.push(rule.clone());
            self.config.rules.len() - 1
        };
        if rule.active_opacity.is_some() || rule.inactive_opacity.is_some() {
            for &h in &affected {
                if let Err(error) = self.opacity.clear(h) {
                    log(&error);
                }
            }
        }
        if rule.mode == Mode::Tile {
            for &h in &affected {
                if window_policy(&self.config, h).mode == Mode::Tile {
                    self.floating.remove(&(h as isize));
                }
            }
        }
        self.apply_rules();
        SendMessageW(
            GetDlgItem(self.settings.hwnd, 42),
            LB_SETCURSEL,
            saved_index,
            0,
        );
        self.settings.fill(Some(&rule));
        affected
            .iter()
            .filter(|&&h| basic_eligible(h) && !elevated(h))
            .count()
    }
    pub(super) unsafe fn window_behavior_reason(&self, h: HWND) -> String {
        if elevated(h) {
            return "Runs elevated; normal-user Omarchy Overlay cannot manage it.".into();
        }
        if self
            .config
            .excluded_processes
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&process_name(h)))
        {
            return "Excluded from management by this profile.".into();
        }
        if !basic_eligible(h) {
            return "Protected dialog, fullscreen or unavailable window.".into();
        }
        match window_policy(&self.config, h).mode {
            Mode::Ignore => "Kept native by the effective Ignore choice.".into(),
            Mode::Float => "Kept floating by the effective app/global choice.".into(),
            Mode::Tile => {
                if !self.active {
                    "Tiling is paused. Enable tiling to arrange it.".into()
                } else if self
                    .scratchpads
                    .values()
                    .any(|p| p.task.hwnd == h && p.task.valid())
                {
                    "Assigned as a scratchpad; it stays floating.".into()
                } else if self.floating.contains(&(h as isize)) {
                    "Manually floating. Save Tile to return this app to tiling.".into()
                } else if self.tiled.values().any(|g| g.keys.contains(&(h as isize))) {
                    "Included in the current tiled arrangement.".into()
                } else {
                    "Waiting for an available tile on this monitor.".into()
                }
            }
        }
    }
}
