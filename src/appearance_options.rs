//! Global appearance, supported materials, motion and reversible Windows preferences.
use super::*;
pub(super) const CLASS: &str = "OmarchyOverlay.AppearanceOptions";
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let h = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — materials, Windows theme and motion").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        920,
        790,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    child(h, "STATIC", "GLOBAL APPEARANCE", 0, 0, 24, 20, 850, 26);
    child(h,"STATIC","Global values apply to eligible apps. Explicit app rules can keep their own opacity, frame light/dark mode and material.",0,0,24,58,850,44);
    for (id, label, x) in [
        (1210, "Active window opacity %", 24),
        (1211, "Inactive window opacity %", 455),
    ] {
        child(h, "STATIC", label, 0, 0, x, 110, 390, 24);
        child(
            h,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            id,
            x,
            140,
            390,
            28,
        );
    }
    child(
        h,
        "STATIC",
        "Material for app windows (Omarchy Overlay menus: set in the menu)",
        0,
        0,
        24,
        194,
        840,
        24,
    );
    let c = child(
        h,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        1201,
        24,
        226,
        390,
        220,
    );
    for s in [
        "Solid palette background",
        "Mica (Windows material)",
        "Acrylic (Windows blur)",
    ] {
        SendMessageW(c, CB_ADDSTRING, 0, wide(s).as_ptr() as isize);
    }
    child(
        h,
        "BUTTON",
        "Coordinate eligible app frame light/dark mode",
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        1202,
        455,
        226,
        410,
        30,
    );
    child(
        h,
        "BUTTON",
        "Animate tile changes",
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        1203,
        24,
        282,
        390,
        30,
    );
    child(
        h,
        "STATIC",
        "Duration in milliseconds (50–400)",
        0,
        0,
        455,
        278,
        410,
        24,
    );
    child(
        h,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        1204,
        455,
        310,
        410,
        28,
    );
    for (id, label, x, y, w) in [
        (1206, "Save and apply", 24, 360, 265),
        (1205, "Use solid readable surfaces", 306, 360, 265),
        (1207, "Per-app appearance…", 588, 360, 277),
        (1208, "Match Windows light/dark", 24, 468, 400),
        (1209, "Restore Windows color preference", 455, 468, 410),
        (1212, "Open Windows color settings", 24, 522, 400),
        (1214, "Close", 455, 522, 410),
    ] {
        child(
            h,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            y,
            w,
            36,
        );
    }
    child(
        h,
        "STATIC",
        "OPTIONAL WINDOWS COLOR PREFERENCE — original values are restored on exit or recovery",
        0,
        0,
        24,
        422,
        840,
        28,
    );
    child(h, "STATIC", "", 0, 1213, 24, 584, 840, 70);
    child(h,"STATIC","Materials use supported Windows surfaces and may fall back to solid rendering. App client content stays native.\r\nOpacity fades the entire window, including text. Solid surfaces and an opaque bar improve contrast.\r\nAnimations respect the Windows animation setting and are disabled during live mouse resizing.",0,0,24,672,840,70);
    unsafe extern "system" fn setfont(h: HWND, f: LPARAM) -> i32 {
        SendMessageW(h, WM_SETFONT, f as usize, 1);
        1
    }
    EnumChildWindows(h, Some(setfont), font as isize);
    h
}
unsafe fn fill(app: &App) {
    let h = app.appearance_options;
    for (id, value) in [
        (1210, app.config.active_opacity.to_string()),
        (1211, app.config.inactive_opacity.to_string()),
        (1204, app.config.animation_duration_ms.to_string()),
    ] {
        set_text(GetDlgItem(h, id), &value);
    }
    SendMessageW(
        GetDlgItem(h, 1201),
        CB_SETCURSEL,
        match app.config.window_backdrop.as_str() {
            "mica" => 1,
            "acrylic" => 2,
            _ => 0,
        },
        0,
    );
    for (id, checked) in [
        (1202, app.config.coordinate_windows),
        (1203, app.config.animations),
    ] {
        SendMessageW(
            GetDlgItem(h, id),
            BM_SETCHECK,
            if checked { BST_CHECKED } else { BST_UNCHECKED } as usize,
            0,
        );
    }
}
pub(super) unsafe fn command(app: &mut App, id: usize) {
    let h = app.appearance_options;
    let result = (|| -> Result<String, String> {
        match id {
            1206 => {
                let number = |id| {
                    text_of(GetDlgItem(h, id))
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| {
                            "Enter whole-number opacity percentages and an animation duration."
                                .to_owned()
                        })
                };
                let active = number(1210)?;
                let inactive = number(1211)?;
                if !(50..=100).contains(&active) || !(70..=100).contains(&inactive) {
                    return Err(
                        "Active opacity must be 50–100%; inactive opacity must be 70–100%.".into(),
                    );
                }
                let mut c = app.config.clone();
                c.active_opacity = active as u8;
                c.inactive_opacity = inactive as u8;
                c.animation_duration_ms = number(1204)?;
                c.window_backdrop = match SendMessageW(GetDlgItem(h, 1201), CB_GETCURSEL, 0, 0) {
                    1 => "mica",
                    2 => "acrylic",
                    _ => "solid",
                }
                .into();
                c.coordinate_windows =
                    SendMessageW(GetDlgItem(h, 1202), BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                c.animations =
                    SendMessageW(GetDlgItem(h, 1203), BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                c.validate()?;
                app.config = c;
                if !app.config.animations {
                    app.finish_motion(true);
                }
                app.apply_own_appearance();
                app.appearance_poll();
                app.update_appearance();
                app.save_config();
                appearance_ui::fill(app);
                Ok("Saved global appearance. App overrides and the Omarchy Overlay menus material are unchanged. Windows supplies materials when supported.".into())
            }
            1205 => {
                app.config.bar_opacity = 100;
                app.config.window_backdrop = "solid".into();
                bar::appearance(100);
                app.apply_own_appearance();
                app.update_appearance();
                app.save_config();
                appearance_ui::fill(app);
                fill(app);
                Ok("Using solid palette backgrounds and an opaque bar for readable text and icons. App-specific choices remain intact.".into())
            }
            1207 => {
                app.show_app_rules();
                Ok("Per-app choices are in Apps and global defaults.".into())
            }
            1208 => {
                windows_personalization::apply(&app.directory, theme::current().dark)?;
                Ok("Windows light/dark mode now follows this palette. Its original preference is saved for restore and exit.".into())
            }
            1209 => {
                windows_personalization::restore(&app.directory)?;
                Ok("Restored the Windows color preference. Frame choices remain controlled by their global/app settings.".into())
            }
            1212 => {
                shell(Path::new("ms-settings:colors"), "");
                Ok("Opened native Windows color settings.".into())
            }
            1214 => {
                ShowWindow(h, SW_HIDE);
                Ok(String::new())
            }
            _ => Ok(String::new()),
        }
    })();
    set_text(
        GetDlgItem(h, 1213),
        &match result {
            Ok(s) => s,
            Err(e) => e,
        },
    );
}
impl App {
    pub(super) unsafe fn show_appearance_options(&self) {
        fill(self);
        let h = self.appearance_options;
        set_text(GetDlgItem(h,1213),"Use per-app appearance for individual exceptions. Optional Windows color coordination has its own restore action.");
        crate::desktops::follow(h);
        panel_scroll::fit(h);
        ShowWindow(h, SW_SHOWNORMAL);
        SetForegroundWindow(h);
    }
    pub(super) unsafe fn apply_own_appearance(&self) {
        for h in [
            self.main,
            self.launcher,
            self.appearance,
            self.settings.hwnd,
            self.profiles,
            self.bar_settings,
            self.module_settings,
            self.tasks.hwnd,
            self.help,
            self.appearance_profiles,
            self.rule_center,
            self.appearance_options,
        ] {
            // 0.15 windows draw GDI text, which has no alpha: blur under them washes the
            // text out, so they stay plain whatever material is picked (0.17, item 3.2).
            window_theme::own(h, "solid");
        }
        overlay::set_material(&self.config.menu_backdrop);
    }
}
