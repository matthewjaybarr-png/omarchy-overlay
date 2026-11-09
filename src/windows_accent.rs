//! The Windows accent color follows the theme, like omarchy-theme-set, with an undo journal.
//! Only DWORD values are written; the binary AccentPalette is left to Windows.
use super::*;
use windows_sys::Win32::System::Registry::*;
const FILE: &str = "windows-accent-restore.json";
const DWM: &str = "Software\\Microsoft\\Windows\\DWM";
const EXPLORER: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Accent";
/// (key, value name): the accent Windows uses for frames, Start and the taskbar.
const VALUES: [(&str, &str); 5] = [
    (DWM, "AccentColor"),
    (DWM, "ColorizationColor"),
    (DWM, "ColorizationAfterglow"),
    (EXPLORER, "AccentColorMenu"),
    (EXPLORER, "StartColorMenu"),
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    before: Option<u32>,
    acceptable: Vec<Option<u32>>,
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
unsafe fn open(path: &str) -> Result<Key, String> {
    let mut h = null_mut();
    let status = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        wide(path).as_ptr(),
        0,
        null(),
        0,
        KEY_QUERY_VALUE | KEY_SET_VALUE,
        null(),
        &mut h,
        null_mut(),
    );
    if status != ERROR_SUCCESS {
        return Err(format!("The Windows accent color is unavailable ({status})."));
    }
    Ok(Key(h))
}
unsafe fn read(h: HKEY, name: &str) -> Result<Option<u32>, String> {
    let (mut value, mut kind, mut size) = (0u32, 0, 4);
    let status = RegQueryValueExW(
        h,
        wide(name).as_ptr(),
        null(),
        &mut kind,
        &mut value as *mut _ as _,
        &mut size,
    );
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS || kind != REG_DWORD || size != 4 {
        return Err("Unexpected Windows accent value; it was left unchanged.".into());
    }
    Ok(Some(value))
}
unsafe fn write(h: HKEY, name: &str, value: Option<u32>) -> Result<(), String> {
    let status = match value {
        Some(v) => RegSetValueExW(h, wide(name).as_ptr(), 0, REG_DWORD, &v as *const _ as _, 4),
        None => RegDeleteValueW(h, wide(name).as_ptr()),
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!("Windows did not allow the accent color change ({status})."))
    }
}
pub(super) unsafe fn notify() {
    let mut result = 0;
    SendMessageTimeoutW(
        HWND_BROADCAST,
        WM_SETTINGCHANGE,
        0,
        wide("ImmersiveColorSet").as_ptr() as isize,
        SMTO_ABORTIFHUNG,
        200,
        &mut result,
    );
}
fn valid(entries: &[Entry]) -> bool {
    entries.len() == VALUES.len()
        && entries.iter().zip(VALUES).all(|(e, (_, n))| e.name == n && e.acceptable.len() <= 2)
}
unsafe fn record(path: &Path) -> Result<Vec<Entry>, String> {
    let entries: Vec<Entry> =
        serde_json::from_str(&profile::read_text(path)?).map_err(|e| e.to_string())?;
    if !valid(&entries) {
        return Err("Invalid Windows accent recovery record; it was preserved.".into());
    }
    Ok(entries)
}
/// Sets the accent (a COLORREF), recording the original values on first use.
pub(super) unsafe fn apply(directory: &Path, accent: u32) -> Result<(), String> {
    let keys = [open(DWM)?, open(EXPLORER)?];
    let key = |k: &str| if k == DWM { keys[0].0 } else { keys[1].0 };
    let path = directory.join(FILE);
    let mut entries = if path.exists() {
        record(&path)?
    } else {
        VALUES
            .iter()
            .map(|&(k, name)| {
                Ok(Entry { name: name.into(), before: read(key(k), name)?, acceptable: vec![] })
            })
            .collect::<Result<Vec<_>, String>>()?
    };
    let next = crate::theme_files::accent_values(accent);
    for (entry, ((k, name), v)) in entries.iter_mut().zip(VALUES.iter().zip(next)) {
        let current = read(key(k), name)?;
        if !entry.acceptable.is_empty()
            && current != entry.before
            && !entry.acceptable.contains(&current)
        {
            return Err("The Windows accent color changed outside Omarchy Overlay; restore or review that change first.".into());
        }
        entry.acceptable = vec![current, Some(v)];
    }
    write_json_atomic(&path, &entries)?;
    for ((k, name), v) in VALUES.iter().zip(next) {
        write(key(k), name, Some(v))?;
    }
    notify();
    Ok(())
}
pub(super) fn pending(directory: &Path) -> bool {
    directory.join(FILE).exists()
}
/// Restores without the settings broadcast; the caller sends one for every color value.
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let path = directory.join(FILE);
    if !path.exists() {
        return Ok(());
    }
    let keys = [open(DWM)?, open(EXPLORER)?];
    let key = |k: &str| if k == DWM { keys[0].0 } else { keys[1].0 };
    let entries = record(&path)?;
    for (entry, &(k, name)) in entries.iter().zip(VALUES.iter()) {
        let current = read(key(k), name)?;
        if current != entry.before && !entry.acceptable.contains(&current) {
            return Err("The Windows accent color changed externally; its recovery record was kept without overwriting it.".into());
        }
    }
    for (entry, &(k, name)) in entries.iter().zip(VALUES.iter()) {
        write(key(k), name, entry.before)?;
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}

