//! Chromium browsers' toolbar color follows the theme, as omarchy-theme-set-browser does:
//! the BrowserThemeColor policy (the theme background) and BrowserColorScheme "device",
//! for Chrome, Edge, Brave and Chromium. Policy keys need administrator rights, so this
//! is for the elevated Admin edition. Browsers then say "Managed by your organization",
//! as they do on Omarchy. The original values are journaled and put back on Remove, on
//! exit and after a crash.
use super::*;
use windows_sys::Win32::System::Registry::*;

const RECORD: &str = "browser-policy-restore.json";
const BROWSERS: [&str; 4] = [
    "Software\\Policies\\Google\\Chrome",
    "Software\\Policies\\Microsoft\\Edge",
    "Software\\Policies\\BraveSoftware\\Brave",
    "Software\\Policies\\Chromium",
];
const VALUES: [&str; 2] = ["BrowserThemeColor", "BrowserColorScheme"];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: String,
    value: String,
    before: Option<String>,
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
        return Err(format!("Browser policies need the Admin edition ({status})."));
    }
    Ok(Key(h))
}
unsafe fn read(key: &Key, name: &str) -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut size = (buffer.len() * 2) as u32;
    let mut kind = 0;
    let status = RegQueryValueExW(key.0, wide(name).as_ptr(), null(), &mut kind, buffer.as_mut_ptr().cast(), &mut size);
    (status == ERROR_SUCCESS && kind == REG_SZ).then(|| {
        let n = (size as usize / 2).min(buffer.len());
        String::from_utf16_lossy(&buffer[..n]).trim_end_matches('\0').to_string()
    })
}
unsafe fn write(key: &Key, name: &str, value: Option<&str>) -> Result<(), String> {
    let status = match value {
        Some(v) => {
            let text = wide(v);
            RegSetValueExW(key.0, wide(name).as_ptr(), 0, REG_SZ, text.as_ptr().cast(), (text.len() * 2) as u32)
        }
        None => RegDeleteValueW(key.0, wide(name).as_ptr()),
    };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!("Windows did not allow the browser policy change ({status})."))
    }
}
fn record(directory: &Path) -> Result<Vec<Entry>, String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        return Ok(vec![]);
    }
    let entries: Vec<Entry> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| format!("{RECORD}: {e}"))?;
    if entries.iter().any(|e| !BROWSERS.contains(&e.key.as_str()) || !VALUES.contains(&e.value.as_str())) {
        return Err(format!("{RECORD} is not valid; it was kept."));
    }
    Ok(entries)
}
/// Sets the browsers' theme color from the theme background (a COLORREF).
pub(super) unsafe fn apply(directory: &Path, background: u32) -> Result<(), String> {
    if !crate::launch_unelevated::process_elevated() {
        return Err("Browser colors need the Admin edition, which runs with administrator rights.".into());
    }
    let color = format!("#{:02x}{:02x}{:02x}", background & 255, (background >> 8) & 255, (background >> 16) & 255);
    let mut entries = record(directory)?;
    for browser in BROWSERS {
        let key = open(browser)?;
        for (name, value) in VALUES.iter().zip([color.as_str(), "device"]) {
            if !entries.iter().any(|e| e.key == browser && e.value == *name) {
                entries.push(Entry { key: browser.into(), value: (*name).into(), before: read(&key, name) });
                write_json_atomic(&directory.join(RECORD), &entries)?;
            }
            write(&key, name, Some(value))?;
        }
    }
    Ok(())
}
/// Puts back every policy value from before.
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let entries = record(directory)?;
    if entries.is_empty() {
        return Ok(());
    }
    if !crate::launch_unelevated::process_elevated() {
        return Err("Browser policies from the Admin edition can only be put back by the Admin edition.".into());
    }
    for entry in &entries {
        let key = open(&entry.key)?;
        write(&key, &entry.value, entry.before.as_deref())?;
    }
    fs::remove_file(directory.join(RECORD)).map_err(|e| e.to_string())
}
