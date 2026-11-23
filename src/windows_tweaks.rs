//! Windows switches a standard user owns: taskbar, File Explorer, snap and mouse
//! focus. Each is the value its own Settings switch writes (HKCU or a documented
//! SystemParametersInfo action). The first original value is journaled and put
//! back on Restore, on exit and after a crash.
use super::*;
use windows_sys::Win32::System::Registry::*;
const FILE: &str = "windows-tweaks-restore.json";
const ADVANCED: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";
const SEARCH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Search";

#[derive(Clone, Copy)]
enum Kind {
    /// A DWORD under HKCU; `absent` is what Windows assumes when the value is missing.
    Reg {
        key: &'static str,
        name: &'static str,
        absent: u32,
    },
    /// SystemParametersInfo with the new value passed in pvParam.
    SpiPointer { get: u32, set: u32 },
    /// SystemParametersInfo with the new value passed in uiParam.
    SpiParam { get: u32, set: u32 },
    /// Minimize and maximize animation (ANIMATIONINFO).
    Animation,
}
pub(super) struct Tweak {
    pub id: &'static str,
    pub label: &'static str,
    kind: Kind,
    on: u32,
    off: u32,
}
const fn advanced(
    id: &'static str,
    label: &'static str,
    name: &'static str,
    on: u32,
    off: u32,
    absent: u32,
) -> Tweak {
    Tweak {
        id,
        label,
        kind: Kind::Reg {
            key: ADVANCED,
            name,
            absent,
        },
        on,
        off,
    }
}
pub(super) const TWEAKS: &[Tweak] = &[
    advanced(
        "taskbar-left",
        "Taskbar icons on the left",
        "TaskbarAl",
        0,
        1,
        1,
    ),
    advanced(
        "task-view-button",
        "Task View button on the taskbar",
        "ShowTaskViewButton",
        1,
        0,
        1,
    ),
    Tweak {
        id: "taskbar-search",
        label: "Search on the taskbar",
        kind: Kind::Reg {
            key: SEARCH,
            name: "SearchboxTaskbarMode",
            absent: 2,
        },
        on: 1,
        off: 0,
    },
    advanced(
        "file-extensions",
        "File Explorer shows file name extensions",
        "HideFileExt",
        0,
        1,
        1,
    ),
    advanced(
        "hidden-files",
        "File Explorer shows hidden files",
        "Hidden",
        1,
        2,
        2,
    ),
    Tweak {
        id: "windows-snap",
        label: "Windows snaps windows dragged to a screen edge",
        kind: Kind::SpiParam {
            get: 0x0082,
            set: 0x0083,
        },
        on: 1,
        off: 0,
    },
    advanced(
        "snap-flyout",
        "Snap layouts on the maximize button",
        "EnableSnapAssistFlyout",
        1,
        0,
        1,
    ),
    advanced(
        "snap-bar",
        "Snap layouts at the top of the screen",
        "EnableSnapBar",
        1,
        0,
        1,
    ),
    advanced(
        "snap-assist",
        "Windows suggests what to snap next",
        "SnapAssist",
        1,
        0,
        1,
    ),
    Tweak {
        id: "focus-follows-mouse",
        label: "The window under the mouse gets focus",
        kind: Kind::SpiPointer {
            get: 0x1000,
            set: 0x1001,
        },
        on: 1,
        off: 0,
    },
    Tweak {
        id: "window-animations",
        label: "Windows animates minimize and maximize",
        kind: Kind::Animation,
        on: 1,
        off: 0,
    },
];

#[repr(C)]
struct AnimationInfo {
    size: u32,
    minimize: i32,
}
unsafe fn read(kind: Kind) -> Result<Option<u32>, String> {
    let mut value = 0u32;
    let ok = match kind {
        Kind::Reg { key, name, .. } => {
            let mut size = 4u32;
            let status = RegGetValueW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                wide(name).as_ptr(),
                RRF_RT_REG_DWORD,
                null_mut(),
                &mut value as *mut _ as _,
                &mut size,
            );
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            status == ERROR_SUCCESS
        }
        Kind::SpiPointer { get, .. } | Kind::SpiParam { get, .. } => {
            SystemParametersInfoW(get, 0, &mut value as *mut _ as _, 0) != 0
        }
        Kind::Animation => {
            let mut info = AnimationInfo {
                size: 8,
                minimize: 0,
            };
            let ok = SystemParametersInfoW(SPI_GETANIMATION, 8, &mut info as *mut _ as _, 0) != 0;
            value = u32::from(info.minimize != 0);
            ok
        }
    };
    if ok {
        Ok(Some(value))
    } else {
        Err("Windows did not report this setting; it was left unchanged.".into())
    }
}
unsafe fn write(kind: Kind, value: Option<u32>) -> Result<(), String> {
    let flags = SPIF_UPDATEINIFILE | SPIF_SENDCHANGE;
    let ok = match (kind, value) {
        (Kind::Reg { key, name, .. }, Some(v)) => {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                wide(key).as_ptr(),
                wide(name).as_ptr(),
                REG_DWORD,
                &v as *const _ as _,
                4,
            ) == ERROR_SUCCESS
        }
        (Kind::Reg { key, name, .. }, None) => {
            let status =
                RegDeleteKeyValueW(HKEY_CURRENT_USER, wide(key).as_ptr(), wide(name).as_ptr());
            status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND
        }
        (Kind::SpiPointer { set, .. }, Some(v)) => {
            SystemParametersInfoW(set, 0, v as usize as _, flags) != 0
        }
        (Kind::SpiParam { set, .. }, Some(v)) => {
            SystemParametersInfoW(set, v, null_mut(), flags) != 0
        }
        (Kind::Animation, Some(v)) => {
            let mut info = AnimationInfo {
                size: 8,
                minimize: v as i32,
            };
            SystemParametersInfoW(SPI_SETANIMATION, 8, &mut info as *mut _ as _, flags) != 0
        }
        // SystemParametersInfo settings always have a value to return to.
        (_, None) => true,
    };
    if !ok {
        return Err("Windows did not allow this change.".into());
    }
    if matches!(kind, Kind::Reg { .. }) {
        // The taskbar re-reads on TraySettings; open Explorer windows on the shell notice.
        let mut result = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            wide("TraySettings").as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            200,
            &mut result,
        );
        windows_sys::Win32::UI::Shell::SHChangeNotify(0x0800_0000, 0, null(), null());
    }
    Ok(())
}
/// Whether the switch is on now; None when Windows would not say.
pub(super) unsafe fn is_on(tweak: &Tweak) -> Option<bool> {
    let absent = match tweak.kind {
        Kind::Reg { absent, .. } => absent,
        _ => tweak.off,
    };
    Some(read(tweak.kind).ok()?.unwrap_or(absent) != tweak.off)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    before: Option<u32>,
    acceptable: Vec<Option<u32>>,
}
impl Entry {
    /// False when someone else changed the setting since Omarchy Overlay last wrote it.
    fn still_ours(&self, current: Option<u32>) -> bool {
        current == self.before || self.acceptable.contains(&current)
    }
}
fn parse(text: &str) -> Result<Vec<Entry>, String> {
    let entries: Vec<Entry> = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if entries.iter().enumerate().any(|(i, e)| {
        !TWEAKS.iter().any(|t| t.id == e.id)
            || entries[..i].iter().any(|old| old.id == e.id)
            || e.acceptable.len() > 2
    }) {
        return Err("Invalid Windows switches recovery record; it was preserved.".into());
    }
    Ok(entries)
}
fn record(directory: &Path) -> Result<Vec<Entry>, String> {
    let path = directory.join(FILE);
    if path.exists() {
        parse(&profile::read_text(&path)?)
    } else {
        Ok(vec![])
    }
}
/// Flips one switch, keeping its first original value for Restore.
pub(super) unsafe fn toggle(directory: &Path, id: &str) -> Result<(), String> {
    let tweak = TWEAKS
        .iter()
        .find(|t| t.id == id)
        .ok_or("Unknown Windows switch.")?;
    let mut entries = record(directory)?;
    let current = read(tweak.kind)?;
    let next = Some(if is_on(tweak) == Some(true) {
        tweak.off
    } else {
        tweak.on
    });
    match entries.iter_mut().find(|e| e.id == id) {
        Some(entry) => {
            if !entry.still_ours(current) {
                return Err(format!(
                    "\"{}\" changed outside Omarchy Overlay; restore or review that change first.",
                    tweak.label
                ));
            }
            entry.acceptable = vec![current, next];
        }
        None => entries.push(Entry {
            id: id.into(),
            before: current,
            acceptable: vec![current, next],
        }),
    }
    write_json_atomic(&directory.join(FILE), &entries)?;
    write(tweak.kind, next)
}
pub(super) fn pending(directory: &Path) -> bool {
    directory.join(FILE).exists()
}
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    if !pending(directory) {
        return Ok(());
    }
    let entries = record(directory)?;
    let kind = |id: &str| TWEAKS.iter().find(|t| t.id == id).map(|t| t.kind);
    for entry in &entries {
        let Some(kind) = kind(&entry.id) else {
            continue;
        };
        if !entry.still_ours(read(kind)?) {
            return Err(format!(
                "Windows switch {} changed outside Omarchy Overlay; its recovery record was kept without overwriting it.",
                entry.id
            ));
        }
    }
    for entry in &entries {
        if let Some(kind) = kind(&entry.id) {
            write(kind, entry.before)?;
        }
    }
    fs::remove_file(directory.join(FILE)).map_err(|e| e.to_string())
}
