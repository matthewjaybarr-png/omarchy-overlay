//! Installs the bundled JetBrains Mono Nerd Fonts for this Windows user, so apps such as
//! Windows Terminal can use Omarchy's font (Omarchy Overlay itself loads them privately).
//! Per-user fonts need no administrator rights: the file goes in
//! %LOCALAPPDATA%\Microsoft\Windows\Fonts and a value under HKCU ...\Fonts names it.
//! Only fonts it adds are journaled, and only those are removed again by Remove, on exit
//! and after a crash; a font the user already had is never touched.
use super::*;
use windows_sys::Win32::System::Registry::*;

const RECORD: &str = "fonts-installed-restore.json";
const KEY: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts";
/// (file beside the exe in fonts\, the registry value Windows lists it under).
const FILES: [(&str, &str); 4] = [
    ("JetBrainsMonoNLNerdFont-Regular.ttf", "JetBrainsMonoNL Nerd Font Regular (TrueType)"),
    ("JetBrainsMonoNLNerdFont-Bold.ttf", "JetBrainsMonoNL Nerd Font Bold (TrueType)"),
    ("JetBrainsMonoNerdFont-Regular.ttf", "JetBrainsMono Nerd Font Regular (TrueType)"),
    ("JetBrainsMonoNerdFont-Bold.ttf", "JetBrainsMono Nerd Font Bold (TrueType)"),
];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    file: PathBuf,
    value: String,
}

fn folder() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Microsoft\\Windows\\Fonts"))
}
fn bundled() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("fonts"))
}
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
    let status = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        wide(KEY).as_ptr(),
        0,
        null(),
        0,
        KEY_QUERY_VALUE | KEY_SET_VALUE,
        null(),
        &mut h,
        null_mut(),
    );
    if status != ERROR_SUCCESS {
        return Err(format!("The Windows font list is unavailable ({status})."));
    }
    Ok(Key(h))
}
unsafe fn has_value(key: &Key, name: &str) -> bool {
    RegQueryValueExW(key.0, wide(name).as_ptr(), null(), null_mut(), null_mut(), null_mut()) == ERROR_SUCCESS
}
unsafe fn announce() {
    let mut result = 0;
    SendMessageTimeoutW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0, SMTO_ABORTIFHUNG, 200, &mut result);
}
fn read_record(directory: &Path) -> Result<Vec<Entry>, String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        return Ok(vec![]);
    }
    let entries: Vec<Entry> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| format!("{RECORD}: {e}"))?;
    if entries.len() > FILES.len() || entries.iter().any(|e| !FILES.iter().any(|(_, v)| *v == e.value)) {
        return Err(format!("{RECORD} is not valid; it was kept."));
    }
    Ok(entries)
}
/// True when the bundled fonts are available to install (the fonts\ folder exists).
pub(super) fn available() -> bool {
    bundled().is_some_and(|b| FILES.iter().all(|(f, _)| b.join(f).is_file()))
}
/// Installs every bundled font this user does not have yet. Returns how many it added.
pub(super) unsafe fn install(directory: &Path) -> Result<usize, String> {
    let source = bundled().filter(|_| available()).ok_or("The fonts folder beside OmarchyOverlay.exe is missing.")?;
    let target = folder().ok_or("The per-user fonts folder is unavailable.")?;
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let key = open()?;
    let mut record = read_record(directory)?;
    let mut added = 0;
    for (file, value) in FILES {
        if has_value(&key, value) {
            continue; // Already installed, by us earlier or by the user.
        }
        let path = target.join(file);
        if !record.iter().any(|e| e.value == value) {
            record.push(Entry { file: path.clone(), value: value.into() });
            write_json_atomic(&directory.join(RECORD), &record)?;
        }
        if !path.is_file() {
            fs::copy(source.join(file), &path).map_err(|e| format!("{file}: {e}"))?;
        }
        let text = wide(&path.to_string_lossy());
        let status = RegSetValueExW(
            key.0,
            wide(value).as_ptr(),
            0,
            REG_SZ,
            text.as_ptr().cast(),
            (text.len() * 2) as u32,
        );
        if status != ERROR_SUCCESS {
            return Err(format!("Windows did not allow the font install ({status})."));
        }
        AddFontResourceW(text.as_ptr());
        added += 1;
    }
    if added > 0 {
        announce();
    }
    Ok(added)
}
/// Removes the fonts Omarchy Overlay installed. A file Windows still holds open is left
/// in the folder, unlisted, and deleted on a later Remove.
pub(super) unsafe fn remove(directory: &Path) -> Result<(), String> {
    let record = read_record(directory)?;
    if record.is_empty() {
        let _ = fs::remove_file(directory.join(RECORD));
        return Ok(());
    }
    let key = open()?;
    let mut left = vec![];
    for entry in record {
        let text = wide(&entry.file.to_string_lossy());
        RemoveFontResourceW(text.as_ptr());
        RegDeleteValueW(key.0, wide(&entry.value).as_ptr());
        if entry.file.exists() && fs::remove_file(&entry.file).is_err() {
            left.push(entry);
        }
    }
    announce();
    if left.is_empty() {
        fs::remove_file(directory.join(RECORD)).map_err(|e| e.to_string())
    } else {
        log("Some fonts were in use; their files are deleted on the next Remove.");
        write_json_atomic(&directory.join(RECORD), &left)
    }
}
