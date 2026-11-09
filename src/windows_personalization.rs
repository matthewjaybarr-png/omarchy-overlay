//! Explicit user-level Windows light/dark coordination with a local undo journal.
//! Transparency and accent-on-Start use the same Personalize values that their
//! Settings switches write, journaled separately so each can be restored.
use super::*;
use windows_sys::Win32::System::Registry::*;
const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
const FILE: &str = "windows-colors-restore.json";
const NAMES: [&str; 2] = ["AppsUseLightTheme", "SystemUsesLightTheme"];
const EFFECTS_FILE: &str = "windows-effects-restore.json";
/// Settings > Personalization > Colors: "Transparency effects" and
/// "Show accent color on Start and taskbar".
pub(super) const EFFECTS: [&str; 2] = ["EnableTransparency", "ColorPrevalence"];
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
unsafe fn open() -> Result<Key, String> {
    let mut h = null_mut();
    let status = RegOpenKeyExW(
        HKEY_CURRENT_USER,
        wide(KEY).as_ptr(),
        0,
        KEY_QUERY_VALUE | KEY_SET_VALUE,
        &mut h,
    );
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Windows color preferences are unavailable ({status})."
        ));
    }
    Ok(Key(h))
}
unsafe fn read(h: HKEY, name: &str) -> Result<Option<u32>, String> {
    let mut value = 0u32;
    let mut kind = 0;
    let mut size = 4;
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
    if status != ERROR_SUCCESS || kind != REG_DWORD || size != 4 || value > 1 {
        return Err("Unexpected Windows color preference; it was left unchanged.".into());
    }
    Ok(Some(value))
}
unsafe fn write(h: HKEY, name: &str, value: Option<u32>) -> Result<(), String> {
    let status = if let Some(v) = value {
        RegSetValueExW(h, wide(name).as_ptr(), 0, REG_DWORD, &v as *const _ as _, 4)
    } else {
        RegDeleteValueW(h, wide(name).as_ptr())
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!(
            "Windows did not allow the color preference change ({status})."
        ))
    }
}
unsafe fn notify() {
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    before: Option<u32>,
    acceptable: Vec<Option<u32>>,
}
unsafe fn record(directory: &Path) -> Result<Vec<Entry>, String> {
    let entries: Vec<Entry> = serde_json::from_str(&profile::read_text(&directory.join(FILE))?)
        .map_err(|e| e.to_string())?;
    if entries.len() != 2
        || entries.iter().any(|e| {
            !NAMES.contains(&e.name.as_str())
                || e.before.is_some_and(|v| v > 1)
                || e.acceptable.len() > 2
                || e.acceptable.iter().any(|v| v.is_some_and(|v| v > 1))
        })
        || entries[0].name == entries[1].name
    {
        return Err("Invalid Windows color recovery record; it was preserved.".into());
    }
    Ok(entries)
}
pub(super) unsafe fn apply(directory: &Path, dark: bool) -> Result<(), String> {
    let h = open()?;
    let path = directory.join(FILE);
    let mut entries = if path.exists() {
        record(directory)?
    } else {
        NAMES
            .iter()
            .map(|name| {
                Ok(Entry {
                    name: (*name).into(),
                    before: read(h.0, name)?,
                    acceptable: vec![],
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    };
    let next = Some(u32::from(!dark));
    for entry in &mut entries {
        let current = read(h.0, &entry.name)?;
        if !entry.acceptable.is_empty()
            && current != entry.before
            && !entry.acceptable.contains(&current)
        {
            return Err("Windows colors changed outside Omarchy Overlay; restore or review the original change first.".into());
        }
        entry.acceptable = vec![current, next];
    }
    write_json_atomic(&path, &entries)?;
    for entry in entries {
        write(h.0, &entry.name, next)?;
    }
    notify();
    Ok(())
}
/// Reads one Personalize value now in effect; None when Windows has no value stored.
pub(super) unsafe fn current(name: &str) -> Option<u32> {
    let h = open().ok()?;
    read(h.0, name).ok().flatten()
}
unsafe fn effects_record(directory: &Path) -> Result<Vec<Entry>, String> {
    let entries: Vec<Entry> =
        serde_json::from_str(&profile::read_text(&directory.join(EFFECTS_FILE))?)
            .map_err(|e| e.to_string())?;
    if entries.len() > EFFECTS.len()
        || entries.iter().enumerate().any(|(i, e)| {
            !EFFECTS.contains(&e.name.as_str())
                || entries[..i].iter().any(|old| old.name == e.name)
                || e.before.is_some_and(|v| v > 1)
                || e.acceptable.len() > 2
                || e.acceptable.iter().any(|v| v.is_some_and(|v| v > 1))
        })
    {
        return Err("Invalid Windows effects recovery record; it was preserved.".into());
    }
    Ok(entries)
}
/// Turns transparency or accent-on-Start on or off, keeping the first original value.
pub(super) unsafe fn apply_effect(directory: &Path, name: &str, on: bool) -> Result<(), String> {
    if !EFFECTS.contains(&name) {
        return Err("Unknown Windows effect.".into());
    }
    let h = open()?;
    let path = directory.join(EFFECTS_FILE);
    let mut entries = if path.exists() {
        effects_record(directory)?
    } else {
        vec![]
    };
    let current = read(h.0, name)?;
    let next = Some(u32::from(on));
    match entries.iter_mut().find(|e| e.name == name) {
        Some(entry) => {
            if current != entry.before && !entry.acceptable.contains(&current) {
                return Err(format!(
                    "{name} changed outside Omarchy Overlay; restore or review that change first."
                ));
            }
            entry.acceptable = vec![current, next];
        }
        None => entries.push(Entry {
            name: name.into(),
            before: current,
            acceptable: vec![current, next],
        }),
    }
    write_json_atomic(&path, &entries)?;
    write(h.0, name, next)?;
    notify();
    Ok(())
}
unsafe fn restore_effects(directory: &Path) -> Result<(), String> {
    let path = directory.join(EFFECTS_FILE);
    if !path.exists() {
        return Ok(());
    }
    let h = open()?;
    let entries = effects_record(directory)?;
    for entry in &entries {
        let current = read(h.0, &entry.name)?;
        if current != entry.before && !entry.acceptable.contains(&current) {
            return Err(format!(
                "{} changed outside Omarchy Overlay; its recovery record was kept without overwriting it.",
                entry.name
            ));
        }
    }
    for entry in entries {
        write(h.0, &entry.name, entry.before)?;
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
pub(super) fn pending(directory: &Path) -> bool {
    directory.join(FILE).exists() || directory.join(EFFECTS_FILE).exists()
}
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let pending = pending(directory);
    let result = restore_quiet(directory);
    if pending {
        notify();
    }
    result
}
/// Restores without the settings broadcast, so a caller restoring several values sends one.
pub(super) unsafe fn restore_quiet(directory: &Path) -> Result<(), String> {
    let effects = restore_effects(directory);
    let colors = restore_colors(directory);
    match (colors, effects) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(a), Err(b)) => Err(format!("{a}\r\n{b}")),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}
unsafe fn restore_colors(directory: &Path) -> Result<(), String> {
    let path = directory.join(FILE);
    if !path.exists() {
        return Ok(());
    }
    let h = open()?;
    let entries = record(directory)?;
    for entry in &entries {
        let current = read(h.0, &entry.name)?;
        if current != entry.before && !entry.acceptable.contains(&current) {
            return Err("Windows colors changed externally; their recovery record was kept without overwriting them.".into());
        }
    }
    for entry in entries {
        write(h.0, &entry.name, entry.before)?;
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
