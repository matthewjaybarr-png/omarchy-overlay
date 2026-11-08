use super::*;
use crate::shortcuts::{self, ACTIONS};

pub(super) unsafe fn create() -> HWND {
    let hwnd = CreateWindowExW(
        WS_EX_TOOLWINDOW,
        wide(HELP_CLASS).as_ptr(),
        wide("Omarchy Overlay — keyboard shortcuts").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VSCROLL,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        900,
        740,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    child(hwnd, "STATIC", "KEYBOARD SHORTCUTS", 0, 0, 24, 20, 840, 24);
    child(
        hwnd,
        "STATIC",
        "Select an action, enter its shortcut, then Apply. Empty shortcuts are disabled.",
        0,
        0,
        24,
        54,
        840,
        24,
    );
    child(
        hwnd,
        "LISTBOX",
        "",
        WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
        501,
        24,
        94,
        430,
        470,
    );
    child(hwnd, "STATIC", "Shortcut", 0, 0, 478, 98, 360, 24);
    child(
        hwnd,
        "EDIT",
        "",
        WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        502,
        478,
        130,
        370,
        30,
    );
    for (id, label, x, width) in [
        (503, "Apply", 478, 112),
        (504, "Use default", 602, 124),
        (505, "Disable", 738, 110),
    ] {
        child(
            hwnd,
            "BUTTON",
            label,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
            id,
            x,
            180,
            width,
            36,
        );
    }
    child(
        hwnd,
        "BUTTON",
        "Omarchy preset",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        506,
        478,
        230,
        176,
        36,
    );
    child(
        hwnd,
        "BUTTON",
        "Undo preset",
        BS_OWNERDRAW as u32 | WS_TABSTOP,
        508,
        670,
        230,
        178,
        36,
    );
    child(
        hwnd,
        "EDIT",
        "",
        ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
        507,
        478,
        280,
        370,
        120,
    );
    child(hwnd,"STATIC","Use Ctrl, Alt, Shift or Win with a key.\r\nExamples: Ctrl+Alt+T, Win+Space, Ctrl+Alt+F11.\r\nF12 is reserved by Windows.\r\n\r\nIndividual choices stay fixed when the shared\r\nmodifier changes. Use default follows it again.",0,0,478,420,370,140);
    child(
        hwnd,
        "EDIT",
        "",
        ES_MULTILINE as u32 | ES_READONLY as u32 | WS_TABSTOP,
        9,
        24,
        590,
        824,
        64,
    );
    hwnd
}
unsafe fn selected(hwnd: HWND) -> usize {
    SendMessageW(GetDlgItem(hwnd, 501), LB_GETCURSEL, 0, 0).max(0) as usize
}
fn text(app: &App, index: usize) -> String {
    if index == ACTIONS.len() {
        app.config.cycle_workspace_shortcut.clone()
    } else {
        shortcuts::chord(index, &app.config.modifiers, &app.config.shortcut_overrides)
    }
}
unsafe fn select(app: &App, index: usize) {
    let chord = text(app, index);
    set_text(GetDlgItem(app.help, 502), &chord);
    let id = BASE as i32
        + if index == ACTIONS.len() {
            38
        } else {
            shortcuts::action_slot(index) as i32
        };
    let status = if chord.is_empty() {
        "Disabled. Other apps can use this shortcut."
    } else if app.hotkeys.contains(&id) || (index == ACTIONS.len() && app.cycle_binding.is_some()) {
        "Registered and ready."
    } else if index == ACTIONS.len() && crate::desktops::snapshot().is_none_or(|s| s.ids.len() < 2)
    {
        "Inactive: at least two observable Windows desktops are needed."
    } else {
        "Unavailable: another action, app or Windows owns this shortcut. Choose another chord."
    };
    set_text(GetDlgItem(app.help, 507), status);
}
pub(super) unsafe fn fill(app: &App, index: Option<usize>) {
    let index = index
        .unwrap_or_else(|| selected(app.help))
        .min(ACTIONS.len());
    let list = GetDlgItem(app.help, 501);
    SendMessageW(list, WM_SETREDRAW, 0, 0);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    for (i, label) in ACTIONS
        .iter()
        .map(|a| a.label)
        .chain(std::iter::once("Cycle next workspace"))
        .enumerate()
    {
        let chord = text(app, i);
        let display = if chord.is_empty() { "Disabled" } else { &chord };
        SendMessageW(
            list,
            LB_ADDSTRING,
            0,
            wide(&format!(
                "{label} — {display}{}",
                if !chord.is_empty()
                    && if i == ACTIONS.len() {
                        app.cycle_binding.is_none()
                    } else {
                        !app.hotkeys
                            .contains(&(BASE as i32 + shortcuts::action_slot(i) as i32))
                    }
                {
                    " [unavailable]"
                } else {
                    ""
                }
            ))
            .as_ptr() as isize,
        );
    }
    SendMessageW(list, LB_SETCURSEL, index, 0);
    SendMessageW(list, WM_SETREDRAW, 1, 0);
    InvalidateRect(list, null(), 1);
    select(app, index);
    let exit = text(app, 5);
    set_text(GetDlgItem(app.help,9),&format!("Restore and exit: {}. The dashboard and tray also provide Restore desktop and exit.\r\nWorkspace shortcuts switch native Windows desktops on all monitors together.",if exit.is_empty(){"keyboard action disabled"}else{&exit}));
}
pub(super) unsafe fn command(app: &mut App, id: usize, notification: usize) {
    let index = selected(app.help).min(ACTIONS.len());
    if id == 501 && notification == LBN_SELCHANGE as usize {
        select(app, index);
        return;
    }
    if id == 506 || id == 508 {
        let result = if id == 506 {
            omarchy_keys::apply(app)
        } else {
            omarchy_keys::restore(app)
        };
        fill(app, Some(index));
        set_text(
            GetDlgItem(app.help, 507),
            &match result {
                Ok(text) => text,
                Err(error) => error,
            },
        );
        return;
    }
    if ![503, 504, 505].contains(&id) {
        return;
    }
    let chord = match id {
        504 if index == ACTIONS.len() => shortcuts::DEFAULT_CYCLE.to_owned(),
        504 => shortcuts::chord(index, &app.config.modifiers, &Default::default()),
        505 => String::new(),
        _ => text_of(GetDlgItem(app.help, 502)).trim().to_owned(),
    };
    let result = (|| -> Result<(), String> {
        if let Some(binding) = shortcuts::parse(&chord)? {
            if let Some(label) = shortcuts::conflict(binding, index, &app.config) {
                return Err(format!(
                    "Already assigned to {label}. Change that action first."
                ));
            }
            let own = if index == ACTIONS.len() {
                app.cycle_binding == Some(binding)
            } else {
                app.hotkeys
                    .contains(&(BASE as i32 + shortcuts::action_slot(index) as i32))
                    && shortcuts::parse(&text(app, index))? == Some(binding)
            };
            if !own {
                let probe = BASE as i32 + 512;
                if RegisterHotKey(
                    app.main,
                    probe,
                    binding.modifiers | MOD_NOREPEAT,
                    binding.key,
                ) == 0
                {
                    return Err(
                        "Another app or Windows owns that shortcut. The previous binding was kept."
                            .into(),
                    );
                }
                UnregisterHotKey(app.main, probe);
            }
        }
        if index == ACTIONS.len() {
            app.config.cycle_workspace_shortcut = chord;
        } else if id == 504 {
            app.config.shortcut_overrides.remove(ACTIONS[index].id);
        } else {
            app.config
                .shortcut_overrides
                .insert(ACTIONS[index].id.into(), chord);
        }
        app.rebind_hotkeys();
        app.save_config();
        fill(app, Some(index));
        set_text(
            GetDlgItem(app.settings.hwnd, 71),
            &app.config.cycle_workspace_shortcut,
        );
        Ok(())
    })();
    if let Err(error) = result {
        set_text(GetDlgItem(app.help, 507), &error);
    }
}
