//! Admin edition: the lock screen (and so the sign-in screen behind it) shows the theme's
//! first background, as Omarchy's lock screen follows its theme. It uses Windows' own
//! lock screen policy (PersonalizationCSP in HKLM), so it needs administrator rights and
//! applies to every user of the PC. The picture is copied to %ProgramData%, where the
//! sign-in screen can read it. The original values are journaled and put back on Remove,
//! on exit and after a crash.
use super::*;
use windows_sys::Win32::System::Registry::*;

const RECORD: &str = "lock-screen-restore.json";
const KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\PersonalizationCSP";
const PATHS: [&str; 2] = ["LockScreenImagePath", "LockScreenImageUrl"];
const STATUS: &str = "LockScreenImageStatus";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    paths: Vec<Option<String>>,
    status: Option<u32>,
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
        HKEY_LOCAL_MACHINE,
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
        return Err(format!("The lock screen needs the Admin edition ({status})."));
    }
    Ok(Key(h))
}
unsafe fn read_text(key: &Key, name: &str) -> Option<String> {
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    let mut kind = 0;
    let ok = RegQueryValueExW(key.0, wide(name).as_ptr(), null(), &mut kind, buffer.as_mut_ptr().cast(), &mut size)
        == ERROR_SUCCESS
        && kind == REG_SZ;
    ok.then(|| String::from_utf16_lossy(&buffer[..(size as usize / 2).min(buffer.len())]).trim_end_matches('\0').to_string())
}
unsafe fn read_number(key: &Key, name: &str) -> Option<u32> {
    let (mut value, mut size, mut kind) = (0u32, 4u32, 0);
    (RegQueryValueExW(key.0, wide(name).as_ptr(), null(), &mut kind, (&mut value as *mut u32).cast(), &mut size)
        == ERROR_SUCCESS
        && kind == REG_DWORD)
        .then_some(value)
}
unsafe fn set_text(key: &Key, name: &str, value: Option<&str>) {
    match value {
        Some(v) => {
            let text = wide(v);
            RegSetValueExW(key.0, wide(name).as_ptr(), 0, REG_SZ, text.as_ptr().cast(), (text.len() * 2) as u32);
        }
        None => {
            RegDeleteValueW(key.0, wide(name).as_ptr());
        }
    }
}
unsafe fn set_number(key: &Key, name: &str, value: Option<u32>) {
    match value {
        Some(v) => {
            RegSetValueExW(key.0, wide(name).as_ptr(), 0, REG_DWORD, (&v as *const u32).cast(), 4);
        }
        None => {
            RegDeleteValueW(key.0, wide(name).as_ptr());
        }
    }
}
fn picture_path() -> PathBuf {
    PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into()))
        .join("OmarchyOverlay")
        .join("lock-screen.jpg")
}
/// Shows the theme's first background on the lock screen.
pub(super) unsafe fn apply(directory: &Path, theme_name: &str) -> Result<(), String> {
    if !crate::launch_unelevated::process_elevated() {
        // Standard edition: this user's lock screen only.
        return user::apply(directory, theme_name);
    }
    let Some(picture) = appearance_effects::backgrounds(theme_name).into_iter().next() else {
        return Err("This theme has no backgrounds for the lock screen.".into());
    };
    let key = open()?;
    let record = directory.join(RECORD);
    if !record.exists() {
        let before = Record {
            paths: PATHS.iter().map(|n| read_text(&key, n)).collect(),
            status: read_number(&key, STATUS),
        };
        write_json_atomic(&record, &before)?;
    }
    let target = picture_path();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(&picture, &target).map_err(|e| format!("{}: {e}", picture.display()))?;
    let path = target.to_string_lossy();
    for name in PATHS {
        set_text(&key, name, Some(&path));
    }
    set_number(&key, STATUS, Some(1));
    Ok(())
}
/// Puts the lock screen settings back as they were.
/// Never fatal for the rest of recovery: a lock screen Windows won't change back is
/// logged and retried at the next exit, and never stops Omarchy Overlay from starting.
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    if let Err(error) = user::restore(directory) {
        log(&format!("Lock screen: {error}; it is tried again at the next exit."));
    }
    let record = directory.join(RECORD);
    if !record.exists() {
        return Ok(());
    }
    if !crate::launch_unelevated::process_elevated() {
        // The Admin edition's all-users lock screen: only it can put that back.
        log("Lock screen: the Admin edition's setting is left for the Admin edition to put back.");
        return Ok(());
    }
    let before: Record = serde_json::from_str(&profile::read_text(&record)?).map_err(|e| format!("{RECORD}: {e}"))?;
    if before.paths.len() != PATHS.len() {
        return Err(format!("{RECORD} is not valid; it was kept."));
    }
    let key = open()?;
    for (name, value) in PATHS.iter().zip(&before.paths) {
        set_text(&key, name, value.as_deref());
    }
    set_number(&key, STATUS, before.status);
    let _ = fs::remove_file(picture_path());
    fs::remove_file(record).map_err(|e| e.to_string())
}

/// The Standard edition's lock screen: this user's own, through Windows'
/// UserProfilePersonalizationSettings, which needs no administrator rights. The picture
/// shown before is saved first and set back on Remove, on exit and after a crash.
pub(super) mod user {
    use super::super::*;
    use ::windows::{
        core::HSTRING,
        Storage::{StorageFile, Streams::DataReader},
        System::UserProfile::{LockScreen, UserProfilePersonalizationSettings},
        Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    };
    const ORIGINAL: &str = "lock-screen-original.jpg";
    /// Written once Windows has taken a new picture: only then is there anything to put back.
    const CHANGED: &str = "lock-screen-changed";

    /// Runs `work` on a thread of its own in the multithreaded apartment, where Windows'
    /// asynchronous calls can be waited for, and waits for it.
    fn on_mta<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
        std::thread::spawn(move || unsafe {
            let started = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            let result = work();
            if started {
                CoUninitialize();
            }
            result
        })
        .join()
        .map_err(|_| "The lock screen change stopped unexpectedly.".to_string())?
    }
    fn set(path: PathBuf) -> Result<(), String> {
        let text = |e: ::windows::core::Error| format!("{} ({:#010X})", e.message(), e.code().0);
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.to_string_lossy().as_ref()))
            .and_then(|o| o.get())
            .map_err(text)?;
        // First the personalization API, then LockScreen itself: Windows editions differ
        // in which one an app outside the Store may use.
        let first = match UserProfilePersonalizationSettings::IsSupported() {
            Ok(true) => match UserProfilePersonalizationSettings::Current()
                .and_then(|s| s.TrySetLockScreenImageAsync(&file))
                .and_then(|o| o.get())
            {
                Ok(true) => return Ok(()),
                Ok(false) => "the personalization API said no".to_string(),
                Err(e) => text(e),
            },
            Ok(false) => "the personalization API is not supported".to_string(),
            Err(e) => text(e),
        };
        match LockScreen::SetImageFileAsync(&file).and_then(|a| a.get()) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!(
                "Windows did not change the lock screen picture ({first}; LockScreen: {}).",
                text(e)
            )),
        }
    }
    /// Copies the picture the lock screen shows now, to put it back later.
    fn save_current(target: PathBuf) -> Result<(), String> {
        let text = |e: ::windows::core::Error| e.message();
        let stream = LockScreen::GetImageStream().map_err(text)?;
        let size = stream.Size().map_err(text)? as u32;
        let reader = DataReader::CreateDataReader(&stream).map_err(text)?;
        reader.LoadAsync(size).and_then(|o| o.get()).map_err(text)?;
        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes).map_err(text)?;
        fs::write(&target, bytes).map_err(|e| e.to_string())
    }
    pub(in super::super) fn apply(directory: &Path, theme_name: &str) -> Result<(), String> {
        let Some(picture) = appearance_effects::backgrounds(theme_name).into_iter().next() else {
            return Err("This theme has no backgrounds for the lock screen.".into());
        };
        let original = directory.join(ORIGINAL);
        let changed = directory.join(CHANGED);
        if !original.exists() {
            let target = original.clone();
            on_mta(move || save_current(target))?;
        }
        match on_mta(move || set(picture)) {
            Ok(()) => fs::write(&changed, b"").map_err(|e| e.to_string()),
            Err(error) => {
                // Nothing changed, so there is nothing to put back later.
                if !changed.exists() {
                    let _ = fs::remove_file(&original);
                }
                Err(error)
            }
        }
    }
    pub(in super::super) fn restore(directory: &Path) -> Result<(), String> {
        let original = directory.join(ORIGINAL);
        let changed = directory.join(CHANGED);
        if !changed.exists() {
            let _ = fs::remove_file(&original);
            return Ok(());
        }
        if !original.exists() {
            let _ = fs::remove_file(&changed);
            return Ok(());
        }
        let path = original.clone();
        on_mta(move || set(path))?;
        let _ = fs::remove_file(&changed);
        fs::remove_file(original).map_err(|e| e.to_string())
    }
}
