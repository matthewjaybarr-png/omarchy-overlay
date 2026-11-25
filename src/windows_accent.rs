//! The Windows accent color follows the theme, like omarchy-theme-set, with an undo journal.
//! The DWORD values and the binary AccentPalette (the shades Start and the taskbar use)
//! have journals of their own.
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
const PALETTE_FILE: &str = "windows-accent-palette-restore.json";
const PALETTE: &str = "AccentPalette";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaletteEntry {
    before: Option<Vec<u8>>,
    acceptable: Vec<Option<Vec<u8>>>,
}
unsafe fn read_palette(h: HKEY) -> Result<Option<Vec<u8>>, String> {
    let (mut value, mut kind, mut size) = ([0u8; 64], 0, 64u32);
    let status = RegQueryValueExW(h, wide(PALETTE).as_ptr(), null(), &mut kind, value.as_mut_ptr(), &mut size);
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS || kind != REG_BINARY || size != 32 {
        return Err("Unexpected Windows accent palette; it was left unchanged.".into());
    }
    Ok(Some(value[..32].to_vec()))
}
unsafe fn write_palette(h: HKEY, value: Option<&[u8]>) -> Result<(), String> {
    let status = match value {
        Some(v) => RegSetValueExW(h, wide(PALETTE).as_ptr(), 0, REG_BINARY, v.as_ptr(), v.len() as u32),
        None => RegDeleteValueW(h, wide(PALETTE).as_ptr()),
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!("Windows did not allow the accent palette change ({status})."))
    }
}
fn palette_record(path: &Path) -> Result<PaletteEntry, String> {
    let entry: PaletteEntry = serde_json::from_str(&profile::read_text(path)?).map_err(|e| e.to_string())?;
    let ok = |v: &Option<Vec<u8>>| v.as_ref().is_none_or(|b| b.len() == 32);
    if entry.acceptable.len() > 2 || !ok(&entry.before) || !entry.acceptable.iter().all(ok) {
        return Err("Invalid Windows accent palette recovery record; it was preserved.".into());
    }
    Ok(entry)
}
/// Sets the shades Start and the taskbar use from the accent, journaled like the rest.
unsafe fn apply_palette(directory: &Path, accent: u32) -> Result<(), String> {
    let key = open(EXPLORER)?;
    let path = directory.join(PALETTE_FILE);
    let current = read_palette(key.0)?;
    let mut entry = if path.exists() {
        palette_record(&path)?
    } else {
        PaletteEntry { before: current.clone(), acceptable: vec![] }
    };
    if !entry.acceptable.is_empty() && current != entry.before && !entry.acceptable.contains(&current) {
        return Err("The Windows accent palette changed outside Omarchy Overlay; it was left as it is.".into());
    }
    let last = current
        .as_ref()
        .map_or([0xf7, 0x63, 0x0c, 0], |c| [c[28], c[29], c[30], c[31]]);
    let next = crate::theme_files::accent_palette(accent, last).to_vec();
    entry.acceptable = vec![current, Some(next.clone())];
    write_json_atomic(&path, &entry)?;
    write_palette(key.0, Some(&next))
}
unsafe fn restore_palette(directory: &Path) -> Result<(), String> {
    let path = directory.join(PALETTE_FILE);
    if !path.exists() {
        return Ok(());
    }
    let key = open(EXPLORER)?;
    let entry = palette_record(&path)?;
    let current = read_palette(key.0)?;
    if current != entry.before && !entry.acceptable.contains(&current) {
        return Err("The Windows accent palette changed externally; its recovery record was kept.".into());
    }
    write_palette(key.0, entry.before.as_deref())?;
    fs::remove_file(path).map_err(|e| e.to_string())
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
    // The palette is extra polish: a problem with it never undoes the accent.
    if let Err(error) = apply_palette(directory, accent) {
        log(&error);
    }
    notify();
    Ok(())
}
pub(super) fn pending(directory: &Path) -> bool {
    directory.join(FILE).exists() || directory.join(PALETTE_FILE).exists()
}
/// Restores without the settings broadcast; the caller sends one for every color value.
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let palette = restore_palette(directory);
    let path = directory.join(FILE);
    if !path.exists() {
        return palette;
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
    fs::remove_file(path).map_err(|e| e.to_string())?;
    palette
}

