use super::*;
use crate::appearance_profiles::{Profile, THEMES};
pub(super) const CLASS: &str = "OmarchyOverlay.AppearanceProfiles";
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay — appearance profiles").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        900,
        800,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    child(hwnd, "STATIC", "APPEARANCE PROFILES", 0, 0, 24, 20, 840, 24);
    child(
        hwnd,
        "LISTBOX",
        "",
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
        801,
        24,
        62,
        280,
        415,
    );
    for (label, id, x, y, w) in [
        ("Name", 802, 330, 62, 520),
        ("Gap", 804, 330, 204, 90),
        ("Bar font", 805, 440, 204, 280),
        ("Font px", 806, 740, 204, 110),
        ("Bar height", 812, 330, 274, 100),
        ("Bar opacity %", 813, 460, 274, 130),
    ] {
        child(hwnd, "STATIC", label, 0, 0, x, y, w, 24);
        child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            id,
            x,
            y + 28,
            w,
            28,
        );
    }
    child(hwnd, "STATIC", "Palette", 0, 0, 330, 132, 520, 24);
    let themes = child(
        hwnd,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        803,
        330,
        160,
        520,
        220,
    );
    for (_, label) in THEMES {
        SendMessageW(themes, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
    }
    child(
        hwnd,
        "BUTTON",
        "Show focus outline on eligible apps",
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        807,
        330,
        348,
        520,
        28,
    );
    for (id, label, x, y) in [
        (808, "Apply appearance", 330, 398),
        (809, "Save named profile", 600, 398),
        (811, "Undo appearance", 330, 444),
        (810, "Delete saved profile", 600, 444),
    ] {
        child(
            hwnd,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            y,
            250,
            36,
        );
    }
    child(
        hwnd,
        "BUTTON",
        "Materials, Windows theme and motion…",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        828,
        330,
        480,
        520,
        32,
    );
    child(
        hwnd,
        "BUTTON",
        "Volume/workspace feedback",
        BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        820,
        24,
        480,
        280,
        28,
    );
    child(
        hwnd,
        "STATIC",
        "OPTIONAL WINDOWS INTEGRATIONS — restored on exit or with Restore",
        0,
        0,
        24,
        512,
        826,
        24,
    );
    child(
        hwnd,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        814,
        24,
        550,
        520,
        28,
    );
    child(
        hwnd,
        "BUTTON",
        "Apply wallpaper path",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        815,
        562,
        546,
        288,
        36,
    );
    child(
        hwnd,
        "BUTTON",
        "Add themed Terminal profile",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        816,
        24,
        600,
        330,
        36,
    );
    child(
        hwnd,
        "BUTTON",
        "Open themed Terminal",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        818,
        370,
        600,
        220,
        36,
    );
    child(
        hwnd,
        "BUTTON",
        "Restore integrations",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        817,
        606,
        600,
        244,
        36,
    );
    child(hwnd, "STATIC", "", 0, 819, 24, 656, 826, 60);
    child(hwnd,"STATIC","Relative wallpaper paths follow the active profile. Terminal integration adds a separate\r\nOmarchy Overlay profile; existing profiles and settings.json remain intact.",0,0,24,722,826,48);
    unsafe extern "system" fn font_child(hwnd: HWND, font: LPARAM) -> i32 {
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        1
    }
    EnumChildWindows(hwnd, Some(font_child), font as isize);
    hwnd
}
unsafe fn values(hwnd: HWND, p: &Profile) {
    for (id, text) in [
        (802, p.name.clone()),
        (804, p.gap.to_string()),
        (805, p.font_family.clone()),
        (806, p.font_size.to_string()),
        (812, p.bar_height.to_string()),
        (813, p.bar_opacity.to_string()),
        (814, p.wallpaper.clone()),
    ] {
        set_text(GetDlgItem(hwnd, id), &text);
    }
    let index = THEMES
        .iter()
        .position(|(name, _)| *name == p.theme)
        .unwrap_or(0);
    SendMessageW(GetDlgItem(hwnd, 803), CB_SETCURSEL, index, 0);
    SendMessageW(
        GetDlgItem(hwnd, 820),
        BM_SETCHECK,
        if p.show_osd {
            BST_CHECKED
        } else {
            BST_UNCHECKED
        } as usize,
        0,
    );
    SendMessageW(
        GetDlgItem(hwnd, 807),
        BM_SETCHECK,
        if p.focus_outline {
            BST_CHECKED
        } else {
            BST_UNCHECKED
        } as usize,
        0,
    );
}
pub(super) unsafe fn fill(app: &App) {
    let list = GetDlgItem(app.appearance_profiles, 801);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for p in crate::appearance_profiles::builtins()
        .iter()
        .chain(&app.config.appearance_profiles)
    {
        SendMessageW(list, LB_ADDSTRING, 0, wide(&p.name).as_ptr() as isize);
    }
    values(
        app.appearance_profiles,
        &Profile::capture(&app.config, "My appearance".into()),
    );
}
unsafe fn read(
    hwnd: HWND,
    advanced: Option<crate::appearance_profiles::Advanced>,
) -> Result<Profile, String> {
    let number = |id| {
        text_of(GetDlgItem(hwnd, id))
            .trim()
            .parse::<i32>()
            .map_err(|_| "Appearance values must be whole numbers.".to_owned())
    };
    let opacity = number(813)?;
    if !(50..=100).contains(&opacity) {
        return Err("Bar opacity must be 50–100.".into());
    }
    let p = Profile {
        advanced,
        name: text_of(GetDlgItem(hwnd, 802)).trim().into(),
        theme: THEMES
            .get(SendMessageW(GetDlgItem(hwnd, 803), CB_GETCURSEL, 0, 0) as usize)
            .ok_or("Select a palette.")?
            .0
            .into(),
        gap: number(804)?,
        font_family: text_of(GetDlgItem(hwnd, 805)).trim().into(),
        font_size: number(806)?,
        bar_height: number(812)?,
        bar_opacity: opacity as u8,
        wallpaper: text_of(GetDlgItem(hwnd, 814)).trim().into(),
        show_osd: SendMessageW(GetDlgItem(hwnd, 820), BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
        focus_outline: SendMessageW(GetDlgItem(hwnd, 807), BM_GETCHECK, 0, 0)
            == BST_CHECKED as isize,
    };
    p.validate()?;
    Ok(p)
}
pub(super) unsafe fn command(app: &mut App, id: usize, notification: usize) {
    let result = (|| -> Result<(), String> {
        match id {
            801 if notification == LBN_SELCHANGE as usize => {
                let index =
                    SendMessageW(GetDlgItem(app.appearance_profiles, 801), LB_GETCURSEL, 0, 0)
                        as usize;
                let builtins = crate::appearance_profiles::builtins();
                if let Some(p) = builtins
                    .iter()
                    .chain(&app.config.appearance_profiles)
                    .nth(index)
                {
                    values(app.appearance_profiles, p);
                }
            }
            808 => {
                let list = GetDlgItem(app.appearance_profiles, 801);
                let index = SendMessageW(list, LB_GETCURSEL, 0, 0);
                let advanced = if index >= 0 {
                    crate::appearance_profiles::builtins()
                        .iter()
                        .chain(&app.config.appearance_profiles)
                        .nth(index as usize)
                        .and_then(|p| p.advanced.clone())
                } else {
                    Some(crate::appearance_profiles::Advanced::capture(&app.config))
                };
                let p = read(app.appearance_profiles, advanced)?;
                if app.appearance_preview.is_none() {
                    app.appearance_preview =
                        Some(Profile::capture(&app.config, "Previous appearance".into()));
                }
                app.activate_appearance(&p)?;
            }
            809 => {
                let p = read(
                    app.appearance_profiles,
                    Some(crate::appearance_profiles::Advanced::capture(&app.config)),
                )?;
                if THEMES
                    .iter()
                    .any(|(_, name)| name.eq_ignore_ascii_case(&p.name))
                {
                    return Err(
                        "Choose a new name for your saved profile; built-in names are reserved."
                            .into(),
                    );
                }
                if let Some(old) = app
                    .config
                    .appearance_profiles
                    .iter_mut()
                    .find(|old| old.name.eq_ignore_ascii_case(&p.name))
                {
                    *old = p;
                } else {
                    if app.config.appearance_profiles.len() >= 64 {
                        return Err("Up to 64 appearance profiles are supported.".into());
                    }
                    app.config.appearance_profiles.push(p);
                }
                app.save_config();
                fill(app);
            }
            810 => {
                let name = text_of(GetDlgItem(app.appearance_profiles, 802));
                app.config
                    .appearance_profiles
                    .retain(|p| !p.name.eq_ignore_ascii_case(name.trim()));
                app.save_config();
                fill(app);
            }
            811 => {
                if let Some(p) = app.appearance_preview.take() {
                    app.activate_appearance(&p)?;
                    fill(app);
                }
            }
            815 => {
                let path = text_of(GetDlgItem(app.appearance_profiles, 814));
                let path = PathBuf::from(path.trim());
                let path = if path.is_absolute() {
                    path
                } else {
                    app.profile_folder().join(path)
                };
                appearance_effects::with_effects(|| appearance_effects::apply_wallpaper(&app.directory, &path))?;
            }
            816 => appearance_effects::apply_terminal(&app.directory, &app.config.theme)?,
            817 => appearance_effects::restore(&app.directory)?,
            818 => {
                if !shell_in(Path::new("wt.exe"), "-p \"Omarchy Overlay\"", None) {
                    return Err("Windows Terminal could not open. Add the themed profile first, then restart Terminal if needed.".into());
                }
            }
            828 => app.show_appearance_options(),
            _ => return Ok(()),
        }
        set_text(GetDlgItem(app.appearance_profiles,819),"Applied. Appearance profiles travel with your settings. Optional integrations remain local and can be restored.");
        Ok(())
    })();
    if let Err(error) = result {
        set_text(GetDlgItem(app.appearance_profiles, 819), &error);
    }
}
impl App {
    pub(super) unsafe fn show_appearance_profiles(&self) {
        fill(self);
        crate::desktops::follow(self.appearance_profiles);
        panel_scroll::fit(self.appearance_profiles);
        ShowWindow(self.appearance_profiles, SW_SHOWNORMAL);
        SetForegroundWindow(self.appearance_profiles);
    }
    pub(super) unsafe fn activate_appearance(&mut self, p: &Profile) -> Result<(), String> {
        let mut candidate = self.config.clone();
        p.apply(&mut candidate)?;
        candidate.validate()?;
        if !p.wallpaper.is_empty() && self.directory.join("wallpaper-restore.json").exists() {
            let path = PathBuf::from(&p.wallpaper);
            let path = if path.is_absolute() {
                path
            } else {
                self.profile_folder().join(path)
            };
            appearance_effects::with_effects(|| appearance_effects::apply_wallpaper(&self.directory, &path))?;
        }
        self.config = candidate;
        let name = self.config.theme.clone();
        self.apply_theme(&name);
        bar::settings(&self.config.bar);
        bar::appearance(self.config.bar_opacity);
        self.dirty = true;
        self.tick();
        self.update_dashboard();
        self.update_appearance();
        self.save_config();
        Ok(())
    }
}
