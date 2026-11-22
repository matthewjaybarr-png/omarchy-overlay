use crate::config::Config;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const FILE_NAME: &str = "overlay-profile.json";
const LIMIT: u64 = 1024 * 1024;
#[derive(Clone)]
pub struct Active {
    pub path: PathBuf,
    pub id: Option<String>,
}
/// Unknown keys are allowed since 0.18 so an older version can read a profile a newer
/// one saved; `save` writes them back untouched (see `keep_unknown`).
#[derive(Serialize, Deserialize)]
pub struct Portable {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub settings: Config,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Route {
    executable: String,
    title_contains: String,
    window_class: String,
    monitor: String,
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    let temporary = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let mut created = false;
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        created = true;
        file.write_all(&data).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() && created {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub fn read_text(path: &Path) -> Result<String, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > LIMIT {
        return Err("Profile is larger than 1 MiB.".into());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(text.trim_start_matches('\u{feff}').into())
}
/// Keys that only ever appear in per-PC recovery records, never in settings.
const RECOVERY_KEYS: [&str; 3] = ["hwnd", "records", "pid"];
pub fn parse(text: &str) -> Result<Portable, String> {
    let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Invalid Omarchy Overlay profile: {e}"))?;
    for object in [Some(&value), value.get("settings")].into_iter().flatten() {
        if let Some(key) = RECOVERY_KEYS.iter().find(|k| object.get(**k).is_some()) {
            return Err(format!(
                "Invalid Omarchy Overlay profile: it holds recovery data ({key})."
            ));
        }
    }
    let profile: Portable = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("Invalid Omarchy Overlay profile: {e}"))?;
    if profile.format != "overlay-profile" || profile.version != 1 {
        return Err("Unsupported profile format or version.".into());
    }
    if profile.id.is_empty()
        || profile.id.len() > 80
        || !profile
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("Invalid profile identifier.".into());
    }
    profile.settings.validate()?;
    if profile.settings.rules.iter().any(|r| r.monitor.is_some()) {
        return Err("Portable profiles must not contain per-PC monitor assignments.".into());
    }
    Ok(profile)
}
pub fn locate(executable: &Path, local: &Path, explicit: Option<PathBuf>) -> Active {
    if let Some(path) = explicit {
        return Active {
            path,
            id: Some(String::new()),
        };
    }
    let adjacent = executable
        .parent()
        .unwrap_or(Path::new("."))
        .join(FILE_NAME);
    if adjacent.exists() {
        Active {
            path: adjacent,
            id: Some(String::new()),
        }
    } else {
        Active {
            path: local.join("config.json"),
            id: None,
        }
    }
}
/// Saves the profile as it was before this version first changes its layout. Profiles
/// older than 0.17 keep their historical backup names; from 0.18 on the name is
/// overlay-profile.pre-<version>.json, written only when this version would add keys.
/// One settings folder for every version on the stick (0.18 note item 2). Found by
/// walking up from the exe, so the drive letter and version folder do not matter.
pub const SETTINGS_FOLDER: &str = "OVERLAY SETTINGS";
/// The repository's sample profile. A copy of it is never offered as the user's settings.
const SAMPLE: &str = include_str!("../profiles/overlay-profile.sample.json");
static SETTINGS: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
/// Records the settings folder in use, if the profile is in one. Called once at startup.
pub fn remember_settings(active: &Active) {
    if let Some(folder) = active.path.parent() {
        if folder.file_name().is_some_and(|n| n == SETTINGS_FOLDER) {
            let _ = SETTINGS.set(folder.to_path_buf());
        }
    }
}
/// The OVERLAY SETTINGS folder in use, or None when running from an adjacent or local profile.
pub fn settings() -> Option<&'static Path> {
    SETTINGS.get().map(PathBuf::as_path)
}
pub enum Found {
    Use(Active),
    /// No settings yet. `folder` is where they belong; `source` is the newest real
    /// profile found in the version folders next to it, if any.
    Missing {
        folder: PathBuf,
        source: Option<PathBuf>,
    },
}
/// `G:\Omarchy Overlay` for `G:\Omarchy Overlay\OmarchyOverlay-0.18.0-windows-x64\Omarchy Overlay\OmarchyOverlay.exe`,
/// and `C:\Users\me\APPS\TOOLS` for a copy at `C:\Users\me\APPS\TOOLS\Omarchy Overlay\OmarchyOverlay.exe`
/// (0.19): the program folder is always named "Omarchy Overlay", on any drive.
pub fn release_root(executable: &Path) -> Option<PathBuf> {
    let folder = executable.parent()?;
    if !folder
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("omarchy overlay"))
    {
        return None;
    }
    let parent = folder.parent()?;
    if is_version_folder(parent) {
        parent.parent().map(Path::to_path_buf)
    } else {
        Some(parent.to_path_buf())
    }
}
/// A release folder such as `OmarchyOverlay-0.18.0-windows-x64` (any letter case).
fn is_version_folder(path: &Path) -> bool {
    path.file_name().is_some_and(|n| {
        n.to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("omarchyoverlay-")
    })
}
/// The nearest OVERLAY SETTINGS folder in the exe's folder or up to three folders above it.
pub fn settings_folder(executable: &Path) -> Option<PathBuf> {
    executable
        .parent()?
        .ancestors()
        .take(4)
        .map(|d| d.join(SETTINGS_FOLDER))
        .find(|d| d.is_dir())
}
/// Where the profile is for this copy. The Admin install sits in a folder only
/// administrators can change, but any user may create folders in the ones above it
/// (`C:\Program Files`, `C:\`), so it never walks up for an OVERLAY SETTINGS folder:
/// its profile is beside it (admin-only) or in the user's own local folder.
pub fn find_for(executable: &Path, local: &Path, explicit: Option<PathBuf>, installed: bool) -> Found {
    if installed {
        return Found::Use(locate(executable, local, explicit));
    }
    find(executable, local, explicit)
}
/// Where the profile is.
pub fn find(executable: &Path, local: &Path, explicit: Option<PathBuf>) -> Found {
    if explicit.is_some() {
        return Found::Use(locate(executable, local, explicit));
    }
    if let Some(folder) = settings_folder(executable) {
        let path = folder.join(FILE_NAME);
        return if path.is_file() {
            Found::Use(Active {
                path,
                id: Some(String::new()),
            })
        } else {
            Found::Missing {
                source: newest_profile(&folder, executable),
                folder,
            }
        };
    }
    let active = locate(executable, local, None);
    match release_root(executable) {
        Some(root) if active.id.is_none() => {
            let folder = root.join(SETTINGS_FOLDER);
            Found::Missing {
                source: newest_profile(&folder, executable),
                folder,
            }
        }
        _ => Found::Use(active),
    }
}
/// The newest valid profile beside this exe or in a release folder (`OmarchyOverlay-*`) next
/// to the settings folder or in ARCHIVED VERSIONS, skipping copies of the sample. Other
/// folders, such as OLD DEFAULTS or SAVED, are never searched (0.19).
pub fn newest_profile(settings: &Path, executable: &Path) -> Option<PathBuf> {
    let root = settings.parent()?;
    let mut candidates: Vec<PathBuf> = executable
        .parent()
        .map(|d| d.join(FILE_NAME))
        .into_iter()
        .collect();
    for folder in [root.to_path_buf(), root.join("ARCHIVED VERSIONS")] {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let dir = entry.path();
            if !is_version_folder(&dir) {
                continue;
            }
            candidates.push(dir.join(FILE_NAME));
            candidates.push(dir.join("Omarchy Overlay").join(FILE_NAME));
        }
    }
    candidates
        .into_iter()
        .filter(|p| p.is_file())
        .filter_map(|p| {
            let text = read_text(&p).ok()?;
            if text.replace("\r\n", "\n").trim() == SAMPLE.replace("\r\n", "\n").trim() {
                return None;
            }
            parse(&text).ok()?;
            let saved = fs::metadata(&p).and_then(|m| m.modified()).ok()?;
            Some((saved, p))
        })
        .max_by_key(|(saved, _)| *saved)
        .map(|(_, p)| p)
}
/// One line about a profile for the first-run question: date, theme and app rules.
pub fn describe(path: &Path) -> String {
    let theme = read_text(path)
        .and_then(|t| parse(&t))
        .map(|p| {
            format!(
                "theme {}, {} app rules",
                p.settings.theme,
                p.settings.rules.len()
            )
        })
        .unwrap_or_default();
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    format!("{} ({size} bytes, {theme})", path.display())
}
/// Creates the settings folder. With a source profile, copies it with the config files
/// the user edited (not config\examples) and the wallpapers from its folder. Never
/// overwrites a file that is already in the settings folder; the source is not changed.
pub fn create_settings(folder: &Path, source: Option<&Path>) -> Result<PathBuf, String> {
    fs::create_dir_all(folder).map_err(|e| format!("Cannot create {}: {e}", folder.display()))?;
    let target = folder.join(FILE_NAME);
    if let Some(source) = source {
        let from = source.parent().unwrap_or(Path::new("."));
        for sub in ["config", "wallpapers"] {
            copy_tree(&from.join(sub), &folder.join(sub), &|p| {
                !(sub == "config" && p.starts_with(from.join("config").join("examples")))
            })?;
        }
        if !target.exists() {
            let text = read_text(source)?;
            parse(&text)?;
            let temporary = folder.join("overlay-profile.json.copying");
            fs::write(&temporary, text).map_err(|e| e.to_string())?;
            fs::rename(&temporary, &target).map_err(|e| e.to_string())?;
        }
    } else if !target.exists() {
        write_json(&target, &portable(&Config::default(), &new_id()))?;
    }
    Ok(target)
}
fn copy_tree(from: &Path, to: &Path, keep: &dyn Fn(&Path) -> bool) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if !keep(&path) {
            continue;
        }
        if path.is_dir() {
            copy_tree(&path, &target, keep)?;
        } else if !target.exists() {
            fs::create_dir_all(to).map_err(|e| e.to_string())?;
            fs::copy(&path, &target).map_err(|e| format!("Cannot copy {}: {e}", path.display()))?;
        }
    }
    Ok(())
}
pub fn backup_before_upgrade(active: &Active, config: &Config) -> Result<(), String> {
    let text = read_text(&active.path)?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let settings = value.get("settings").unwrap_or(&value);
    let current = format!("pre-{}.json", env!("CARGO_PKG_VERSION"));
    let extension = if settings.get("cycle_workspace_shortcut").is_none() {
        "pre-0.10.json"
    } else if settings.get("applications").is_none() || settings.get("shortcut_overrides").is_none()
    {
        "pre-0.11.json"
    } else if settings.get("appearance_profiles").is_none()
        || settings.get("focus_outline").is_none()
    {
        "pre-0.12.json"
    } else if settings.get("default_mode").is_none() {
        "pre-0.13.json"
    } else if settings.get("scrolling_columns").is_none() {
        "pre-0.14.json"
    } else if settings.get("keymap").is_none() {
        "pre-0.16.json"
    } else {
        let saved = match &active.id {
            Some(id) => kept_value(
                &active.path,
                &portable(config, id),
                &portable(&Config::default(), id),
            ),
            None => kept_value(&active.path, config, &Config::default()),
        }?;
        if !adds_keys(&value, &saved) {
            return Ok(());
        }
        current.as_str()
    };
    keep_copy(&active.path, extension, &text)
}
/// Writes `text` to `path` with `extension` (e.g. `overlay-profile.pre-0.19.0.json`)
/// unless that copy already exists: the first copy is the one an older version can read.
fn keep_copy(path: &Path, extension: &str, text: &str) -> Result<(), String> {
    let backup = path.with_extension(extension);
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(format!("Cannot keep {}: {error}", backup.display())),
    };
    if let Err(error) = file
        .write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(&backup);
        return Err(format!("Cannot keep {}: {error}", backup.display()));
    }
    Ok(())
}
fn routes_path(local: &Path, id: &str) -> PathBuf {
    local.join(format!("profile-monitors-{id}.json"))
}
pub fn load(active: &mut Active, local: &Path) -> Result<Config, String> {
    if active.id.is_some() {
        let profile = parse(&read_text(&active.path)?)?;
        let mut settings = profile.settings;
        let path = routes_path(local, &profile.id);
        if path.exists() {
            let routes: Vec<Route> = serde_json::from_str(&read_text(&path)?)
                .map_err(|e| format!("Invalid per-PC monitor preferences: {e}"))?;
            for rule in &mut settings.rules {
                if let Some(route) = routes.iter().find(|r| {
                    r.executable.eq_ignore_ascii_case(&rule.executable)
                        && r.title_contains == rule.title_contains
                        && r.window_class == rule.window_class
                }) {
                    rule.monitor = Some(route.monitor.clone());
                }
            }
        }
        settings.validate()?;
        active.id = Some(profile.id);
        Ok(settings)
    } else {
        if !active.path.exists() {
            write_json(&active.path, &Config::default())?;
        }
        let settings: Config = serde_json::from_str(&read_text(&active.path)?)
            .map_err(|e| format!("Invalid local settings: {e}"))?;
        settings.validate()?;
        Ok(settings)
    }
}
pub fn portable(config: &Config, id: &str) -> Portable {
    let mut settings = config.clone();
    for rule in &mut settings.rules {
        rule.monitor = None;
    }
    Portable {
        format: "overlay-profile".into(),
        version: 1,
        id: id.into(),
        settings,
    }
}
pub fn new_id() -> String {
    format!(
        "profile-{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}
pub fn save(active: &Active, local: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    if let Some(id) = &active.id {
        let routes: Vec<_> = config
            .rules
            .iter()
            .filter_map(|r| {
                r.monitor.as_ref().map(|m| Route {
                    executable: r.executable.clone(),
                    title_contains: r.title_contains.clone(),
                    window_class: r.window_class.clone(),
                    monitor: m.clone(),
                })
            })
            .collect();
        let route_path = routes_path(local, id);
        let previous: Option<Vec<Route>> = if route_path.exists() {
            Some(serde_json::from_str(&read_text(&route_path)?).map_err(|e| e.to_string())?)
        } else {
            None
        };
        write_json(&route_path, &routes)?;
        if let Err(error) = write_kept(
            &active.path,
            &portable(config, id),
            &portable(&Config::default(), id),
        ) {
            let rollback = match previous {
                Some(previous) => write_json(&route_path, &previous),
                None => fs::remove_file(&route_path).map_err(|e| e.to_string()),
            };
            return Err(if let Err(rollback) = rollback {
                format!("{error}; per-PC preference rollback also failed: {rollback}")
            } else {
                error
            });
        }
        Ok(())
    } else {
        write_kept(&active.path, config, &Config::default())
    }
}
/// True when `new` has an object key that `old` lacks, at any depth (arrays by position).
fn adds_keys(old: &serde_json::Value, new: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (old, new) {
        (Value::Object(o), Value::Object(n)) => n
            .iter()
            .any(|(k, v)| o.get(k).is_none_or(|ov| adds_keys(ov, v))),
        (Value::Array(o), Value::Array(n)) => o.iter().zip(n).any(|(a, b)| adds_keys(a, b)),
        _ => false,
    }
}
/// Settings whose value is a map of the user's own entries, not a group of settings.
const MAPS: [&str; 2] = ["shortcut_overrides", "module_display"];
/// Copies keys this version does not know from `old` into `new`, at any depth. Array
/// items are matched by position only when the array kept its length, so a deleted or
/// reordered rule never inherits another rule's keys.
fn keep_unknown(old: &serde_json::Value, new: &mut serde_json::Value) {
    use serde_json::Value;
    match (old, new) {
        (Value::Object(o), Value::Object(n)) => {
            for (k, ov) in o {
                match n.get_mut(k) {
                    // Maps hold the user's own entries: a removed one must stay removed.
                    Some(_) if MAPS.contains(&k.as_str()) => {}
                    Some(nv) => keep_unknown(ov, nv),
                    None => {
                        n.insert(k.clone(), ov.clone());
                    }
                }
            }
        }
        (Value::Array(o), Value::Array(n)) if o.len() == n.len() => {
            for (ov, nv) in o.iter().zip(n.iter_mut()) {
                keep_unknown(ov, nv);
            }
        }
        _ => {}
    }
}
/// Writes settings, keeping keys a newer version saved in the file (0.18 note item 4).
/// Leaves out keys this version added that are not in the file and still have their
/// default value, so an older version that refuses unknown keys can still read the
/// profile until the user actually changes one of the new settings.
fn skip_new_defaults(old: &Value, new: &mut Value, default: &Value) {
    if let (Value::Object(old), Value::Object(new), Value::Object(default)) = (old, new, default) {
        new.retain(|key, value| old.contains_key(key) || default.get(key) != Some(value));
        for (key, value) in new.iter_mut() {
            if let (Some(old), Some(default)) = (old.get(key), default.get(key)) {
                if !MAPS.contains(&key.as_str()) {
                    skip_new_defaults(old, value, default);
                }
            }
        }
    }
}
/// What `save` writes over the file at `path`: unknown keys kept, new default keys left
/// out. Falls back to the full value if leaving keys out would not load back the same.
fn kept_value<T: Serialize + serde::de::DeserializeOwned>(
    path: &Path,
    value: &T,
    default: &T,
) -> Result<Value, String> {
    let full = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let Ok(old) =
        read_text(path).and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string()))
    else {
        return Ok(full);
    };
    let mut new = full.clone();
    let default = serde_json::to_value(default).map_err(|e| e.to_string())?;
    skip_new_defaults(&old, &mut new, &default);
    let same = serde_json::from_value::<T>(new.clone())
        .ok()
        .and_then(|t| serde_json::to_value(t).ok())
        .is_some_and(|v| v == full);
    let mut kept = if same { new } else { full };
    keep_unknown(&old, &mut kept);
    Ok(kept)
}
/// Saves like `kept_value`. The first time a save adds a key the file did not have (so an
/// older version may refuse it), the file as it was is kept as `<name>.pre-<version>.json`
/// first (0.18 review question 2: the copy was only taken at start before).
fn write_kept<T: Serialize + serde::de::DeserializeOwned>(
    path: &Path,
    value: &T,
    default: &T,
) -> Result<(), String> {
    let kept = kept_value(path, value, default)?;
    if let Ok(text) = read_text(path) {
        if let Ok(old) = serde_json::from_str::<Value>(&text) {
            if adds_keys(&old, &kept) {
                keep_copy(
                    path,
                    &format!("pre-{}.json", env!("CARGO_PKG_VERSION")),
                    &text,
                )?;
            }
        }
    }
    write_json(path, &kept)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::AppRule;
    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join(new_id());
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn installed_exe_without_settings_folder_uses_local_profile() {
        let root = temp();
        let exe = root.join("Program Files").join("OmarchyOverlay").join("OmarchyOverlay.exe");
        let local = root.join("local");
        match find(&exe, &local, None) {
            Found::Use(active) => assert_eq!(active.path, local.join("config.json")),
            Found::Missing { .. } => panic!("an installed exe must not ask for a settings folder"),
        }
    }
    #[test]
    fn installed_exe_ignores_a_settings_folder_another_user_could_create() {
        let root = temp();
        let exe = root.join("Program Files").join("OmarchyOverlay").join("OmarchyOverlay.exe");
        let planted = root.join(SETTINGS_FOLDER);
        fs::create_dir_all(&planted).unwrap();
        fs::write(planted.join(FILE_NAME), "{}").unwrap();
        let local = root.join("local");
        match find(&exe, &local, None) {
            Found::Use(active) => assert_eq!(active.path, planted.join(FILE_NAME)),
            Found::Missing { .. } => panic!("a portable copy still finds the folder"),
        }
        match find_for(&exe, &local, None, true) {
            Found::Use(active) => assert_eq!(active.path, local.join("config.json")),
            Found::Missing { .. } => panic!("the installed copy must not ask for a settings folder"),
        }
    }
    #[test]
    fn settings_folder_beside_exe_on_a_fixed_drive_is_used() {
        let root = temp();
        let exe = root.join("tools").join("OmarchyOverlay.exe");
        let folder = exe.parent().unwrap().join(SETTINGS_FOLDER);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(FILE_NAME), "{}").unwrap();
        match find(&exe, &root.join("local"), None) {
            Found::Use(active) => assert_eq!(active.path, folder.join(FILE_NAME)),
            Found::Missing { .. } => panic!("the settings folder exists"),
        }
    }
    #[test]
    fn version_010_profile_has_its_own_rollback_backup() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "old-010")).unwrap();
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("applications");
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("shortcut_overrides");
        let original = value.to_string();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let config = load(&mut active, &root).unwrap();
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.11.json")).unwrap(),
            original
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn version_012_migration_keeps_an_exact_backup_and_expands_legacy_tasks_once() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "old-012")).unwrap();
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("default_mode");
        value["settings"]["bar"]
            .as_object_mut()
            .unwrap()
            .remove("split_task_modules");
        value["settings"]["bar"]["left"] = serde_json::json!(["tasks"]);
        let original = value.to_string();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let config = load(&mut active, &root).unwrap();
        assert_eq!(
            config.bar.left,
            vec![
                crate::bar_model::Module::AppDrawer,
                crate::bar_model::Module::Tasks
            ]
        );
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.13.json")).unwrap(),
            original
        );
        assert_eq!(load(&mut active, &root).unwrap().bar.left, config.bar.left);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn version_016_upgrade_keeps_the_original_profile_for_rollback() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut old = serde_json::to_value(portable(&Config::default(), "old-015")).unwrap();
        for key in [
            "keymap",
            "tap_super",
            "menu_font",
            "menu_font_size",
            "menu_backdrop",
            "start_tiling",
        ] {
            old["settings"].as_object_mut().unwrap().remove(key);
        }
        let original = serde_json::to_string(&old).unwrap();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let mut config = load(&mut active, &root).unwrap();
        backup_before_upgrade(&active, &config).unwrap();
        config.bar.left.push(crate::bar_model::Module::Tiling);
        save(&active, &root, &config).unwrap();
        backup_before_upgrade(&active, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.16.json")).unwrap(),
            original
        );
        assert!(load(&mut active, &root)
            .unwrap()
            .bar
            .left
            .contains(&crate::bar_model::Module::Tiling));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn version_013_profile_keeps_a_rollback_copy_before_scrolling_fields() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "old-013")).unwrap();
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("scrolling_columns");
        let original = value.to_string();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let config = load(&mut active, &root).unwrap();
        assert_eq!(config.scrolling_columns, 2);
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.14.json")).unwrap(),
            original
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn version_011_backup_preserves_the_first_rollback_profile() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "old-011")).unwrap();
        for field in [
            "appearance_profiles",
            "appearance_wallpaper",
            "focus_outline",
            "show_osd",
        ] {
            value["settings"].as_object_mut().unwrap().remove(field);
        }
        let original = value.to_string();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let config = load(&mut active, &root).unwrap();
        assert!(!config.focus_outline);
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        backup_before_upgrade(&active, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.12.json")).unwrap(),
            original
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn old_profiles_keep_settings_and_a_rollback_copy_before_upgrade() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(
            &Config {
                gap: 23,
                ..Config::default()
            },
            "legacy",
        ))
        .unwrap();
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("cycle_workspace_shortcut");
        let original = value.to_string();
        fs::write(&path, &original).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let config = load(&mut active, &root).unwrap();
        assert_eq!(config.gap, 23);
        assert_eq!(
            config.cycle_workspace_shortcut,
            crate::shortcuts::DEFAULT_CYCLE
        );
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        backup_before_upgrade(&active, &config).unwrap();
        assert_eq!(
            fs::read_to_string(path.with_extension("pre-0.10.json")).unwrap(),
            original
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn profiles_travel_without_monitor_or_recovery_state() {
        let root = temp();
        let pc_a = root.join("a");
        let pc_b = root.join("b");
        fs::create_dir_all(&pc_a).unwrap();
        fs::create_dir_all(&pc_b).unwrap();
        let config = Config {
            bar: crate::bar_model::BarConfig {
                workspace_labels: vec![
                    "CAD".into(),
                    "Code".into(),
                    "Office".into(),
                    "Files".into(),
                ],
                ..Default::default()
            },
            gap: 17,
            rules: vec![AppRule {
                executable: "editor.exe".into(),
                monitor: Some("\\\\.\\DISPLAY4".into()),
                ..AppRule::default()
            }],
            ..Config::default()
        };
        let mut active = Active {
            path: root.join(FILE_NAME),
            id: Some("test-profile".into()),
        };
        save(&active, &pc_a, &config).unwrap();
        let text = read_text(&active.path).unwrap();
        assert!(!text.contains("DISPLAY4"));
        assert!(!text.contains("hwnd"));
        assert_eq!(
            load(&mut active, &pc_a).unwrap().rules[0]
                .monitor
                .as_deref(),
            Some("\\\\.\\DISPLAY4")
        );
        let elsewhere = load(&mut active, &pc_b).unwrap();
        assert_eq!(elsewhere.gap, 17);
        assert_eq!(
            elsewhere.bar.workspace_labels,
            ["CAD", "Code", "Office", "Files"]
        );
        assert!(elsewhere.rules[0].monitor.is_none());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn rejects_recovery_data_bad_versions_unknown_fields_and_paths() {
        assert!(parse(r#"{"records":[],"hwnd":123}"#).is_err());
        let mut file = portable(&Config::default(), "ok");
        file.version = 2;
        assert!(parse(&serde_json::to_string(&file).unwrap()).is_err());
        file.version = 1;
        file.id = "../bad".into();
        assert!(parse(&serde_json::to_string(&file).unwrap()).is_err());
        let mut value = serde_json::to_value(portable(&Config::default(), "ok")).unwrap();
        value["settings"]["hwnd"] = 123.into();
        assert!(parse(&value.to_string()).is_err());
    }
    #[test]
    fn incomplete_temporary_write_does_not_replace_last_good_profile() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let active = Active {
            path: path.clone(),
            id: Some("good".into()),
        };
        save(&active, &root, &Config::default()).unwrap();
        fs::write(path.with_extension("json.tmp"), b"{broken").unwrap();
        assert_eq!(
            parse(&read_text(&path).unwrap()).unwrap().settings.gap,
            Config::default().gap
        );
        let config = Config {
            gap: 18,
            ..Config::default()
        };
        save(&active, &root, &config).unwrap();
        assert_eq!(parse(&read_text(&path).unwrap()).unwrap().settings.gap, 18);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn failed_profile_replacement_rolls_back_per_pc_monitor_changes() {
        let root = temp();
        let local = root.join("local");
        fs::create_dir(&local).unwrap();
        let mut config = Config {
            rules: vec![AppRule {
                executable: "editor.exe".into(),
                monitor: Some("\\\\.\\DISPLAY1".into()),
                ..AppRule::default()
            }],
            ..Config::default()
        };
        let mut good = Active {
            path: root.join(FILE_NAME),
            id: Some("rollback-test".into()),
        };
        save(&good, &local, &config).unwrap();
        let blocked = root.join("directory-not-a-file");
        fs::create_dir(&blocked).unwrap();
        config.rules[0].monitor = Some("\\\\.\\DISPLAY4".into());
        assert!(save(
            &Active {
                path: blocked,
                id: good.id.clone()
            },
            &local,
            &config
        )
        .is_err());
        assert_eq!(
            load(&mut good, &local).unwrap().rules[0].monitor.as_deref(),
            Some("\\\\.\\DISPLAY1")
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn corrupt_portable_profile_never_silently_falls_back_to_local() {
        let root = temp();
        let exe = root.join("OmarchyOverlay.exe");
        fs::write(root.join(FILE_NAME), b"{broken").unwrap();
        let mut active = locate(&exe, &root, None);
        assert!(load(&mut active, &root).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_keys_from_a_newer_version_survive_a_save() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "newer")).unwrap();
        value["future_top"] = serde_json::json!({"a": 1});
        value["settings"]["future_setting"] = serde_json::json!("kept");
        value["settings"]["bar"]["future_bar"] = serde_json::json!(7);
        value["settings"]["shortcut_overrides"] = serde_json::json!({});
        fs::write(&path, value.to_string()).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let mut config = load(&mut active, &root).unwrap();
        config.gap = 31;
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["future_top"]["a"], 1);
        assert_eq!(saved["settings"]["future_setting"], "kept");
        assert_eq!(saved["settings"]["bar"]["future_bar"], 7);
        assert_eq!(saved["settings"]["gap"], 31);
        assert!(!path
            .with_extension(format!("pre-{}.json", env!("CARGO_PKG_VERSION")))
            .exists());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn removed_map_entries_stay_removed() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut config = Config::default();
        config.shortcut_overrides.insert("x".into(), "y".into());
        let mut value = serde_json::to_value(portable(&config, "maps")).unwrap();
        value["settings"]["shortcut_overrides"] = serde_json::json!({"gone": "Ctrl+Alt+Q"});
        fs::write(&path, value.to_string()).unwrap();
        let active = Active {
            path: path.clone(),
            id: Some("maps".into()),
        };
        save(&active, &root, &Config::default()).unwrap();
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            saved["settings"]["shortcut_overrides"],
            serde_json::json!({})
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn version_change_that_adds_keys_keeps_a_pre_version_copy() {
        let root = temp();
        let path = root.join(FILE_NAME);
        let mut value = serde_json::to_value(portable(&Config::default(), "older")).unwrap();
        value["settings"].as_object_mut().unwrap().remove("gap");
        fs::write(&path, value.to_string()).unwrap();
        let mut active = locate(&root.join("OmarchyOverlay.exe"), &root, None);
        let mut config = load(&mut active, &root).unwrap();
        let backup = path.with_extension(format!("pre-{}.json", env!("CARGO_PKG_VERSION")));
        // A new key at its default value is left out, so nothing changes yet.
        backup_before_upgrade(&active, &config).unwrap();
        save(&active, &root, &config).unwrap();
        assert!(!backup.exists());
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(saved["settings"].get("gap").is_none());
        let before_change = fs::read_to_string(&path).unwrap();
        // Once the setting is changed the layout changes, and the save itself keeps the
        // old file first (0.18 review question 2: no restart needed).
        config.gap = Config::default().gap + 5;
        save(&active, &root, &config).unwrap();
        assert_eq!(fs::read_to_string(&backup).unwrap(), before_change);
        // Later saves never replace that first copy.
        config.gap += 1;
        save(&active, &root, &config).unwrap();
        assert_eq!(fs::read_to_string(&backup).unwrap(), before_change);
        assert_eq!(
            load(&mut active, &root).unwrap().gap,
            Config::default().gap + 6
        );
        let _ = fs::remove_dir_all(root);
    }
    fn release(root: &Path, version: &str) -> PathBuf {
        let folder = root
            .join(format!("OmarchyOverlay-{version}-windows-x64"))
            .join("Omarchy Overlay");
        fs::create_dir_all(&folder).unwrap();
        folder.join("OmarchyOverlay.exe")
    }
    #[test]
    fn a_fresh_release_finds_the_shared_settings_folder() {
        let root = temp();
        let settings = root.join(SETTINGS_FOLDER);
        fs::create_dir_all(&settings).unwrap();
        fs::write(
            settings.join(FILE_NAME),
            serde_json::to_string(&portable(&Config::default(), "user")).unwrap(),
        )
        .unwrap();
        let exe = release(&root, "0.18.0");
        match find(&exe, &root.join("local"), None) {
            Found::Use(active) => assert_eq!(active.path, settings.join(FILE_NAME)),
            Found::Missing { .. } => panic!("settings not found"),
        }
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn missing_settings_offer_the_newest_real_profile_not_the_sample() {
        let root = temp();
        let live = root
            .join("ARCHIVED VERSIONS")
            .join("OmarchyOverlay-0.16.0-windows-x64")
            .join("Omarchy Overlay");
        fs::create_dir_all(live.join("wallpapers")).unwrap();
        fs::create_dir_all(live.join("config").join("examples")).unwrap();
        let user = Config {
            gap: 19,
            ..Config::default()
        };
        fs::write(
            live.join(FILE_NAME),
            serde_json::to_string(&portable(&user, "old-work")).unwrap(),
        )
        .unwrap();
        fs::write(live.join("wallpapers").join("w.png"), b"png").unwrap();
        fs::write(live.join("config").join("binds.conf"), b"# mine").unwrap();
        fs::write(
            live.join("config").join("examples").join("binds.conf"),
            b"# example",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let sample = root
            .join("OmarchyOverlay-0.17.0-windows-x64")
            .join("Omarchy Overlay");
        fs::create_dir_all(&sample).unwrap();
        fs::write(sample.join(FILE_NAME), SAMPLE).unwrap();
        let exe = release(&root, "0.18.0");
        let Found::Missing { folder, source } = find(&exe, &root.join("local"), None) else {
            panic!("expected missing settings");
        };
        assert_eq!(folder, root.join(SETTINGS_FOLDER));
        assert_eq!(source.as_deref(), Some(live.join(FILE_NAME).as_path()));
        let original = fs::read_to_string(live.join(FILE_NAME)).unwrap();
        let path = create_settings(&folder, source.as_deref()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert!(folder.join("wallpapers").join("w.png").exists());
        assert!(folder.join("config").join("binds.conf").exists());
        assert!(!folder.join("config").join("examples").exists());
        // A second fill never overwrites what is already there.
        fs::write(&path, "changed").unwrap();
        create_settings(&folder, source.as_deref()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "changed");
        assert_eq!(fs::read_to_string(live.join(FILE_NAME)).unwrap(), original);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn search_skips_folders_that_are_not_releases() {
        let root = temp();
        let release016 = root
            .join("ARCHIVED VERSIONS")
            .join("OmarchyOverlay-0.16.0-windows-x64")
            .join("Omarchy Overlay");
        fs::create_dir_all(&release016).unwrap();
        fs::write(
            release016.join(FILE_NAME),
            serde_json::to_string(&portable(
                &Config {
                    gap: 19,
                    ..Config::default()
                },
                "work",
            ))
            .unwrap(),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Newer, but in OLD DEFAULTS and SAVED: never offered.
        for other in ["OLD DEFAULTS", "SAVED"] {
            let dir = root.join(other).join("Omarchy Overlay");
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join(FILE_NAME),
                serde_json::to_string(&portable(
                    &Config {
                        gap: 3,
                        ..Config::default()
                    },
                    "x",
                ))
                .unwrap(),
            )
            .unwrap();
            fs::write(
                root.join(other).join(FILE_NAME),
                serde_json::to_string(&portable(
                    &Config {
                        gap: 4,
                        ..Config::default()
                    },
                    "y",
                ))
                .unwrap(),
            )
            .unwrap();
        }
        let exe = release(&root, "0.19.0");
        let Found::Missing { source, .. } = find(&exe, &root.join("local"), None) else {
            panic!("expected missing settings");
        };
        assert_eq!(
            source.as_deref(),
            Some(release016.join(FILE_NAME).as_path())
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn a_pc_copy_in_any_folder_uses_the_settings_folder_beside_it() {
        // C:\Users\me\APPS\TOOLS\Omarchy Overlay\OmarchyOverlay.exe with TOOLS\OVERLAY SETTINGS.
        let root = temp().join("APPS").join("TOOLS");
        let exe = root.join("Omarchy Overlay").join("OmarchyOverlay.exe");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        assert_eq!(release_root(&exe).as_deref(), Some(root.as_path()));
        // No settings folder yet, on a fixed drive: it asks instead of using local mode.
        match find(&exe, &root.join("local"), None) {
            Found::Missing { folder, source } => {
                assert_eq!(folder, root.join(SETTINGS_FOLDER));
                assert!(source.is_none());
            }
            Found::Use(_) => panic!("a PC copy without settings should ask"),
        }
        let settings = root.join(SETTINGS_FOLDER);
        fs::create_dir_all(&settings).unwrap();
        fs::write(
            settings.join(FILE_NAME),
            serde_json::to_string(&portable(&Config::default(), "user")).unwrap(),
        )
        .unwrap();
        match find(&exe, &root.join("local"), None) {
            Found::Use(active) => assert_eq!(active.path, settings.join(FILE_NAME)),
            Found::Missing { .. } => panic!("settings not found"),
        }
        let _ = fs::remove_dir_all(root.parent().unwrap().parent().unwrap());
    }
    #[test]
    fn development_folders_keep_local_mode() {
        let root = temp();
        let exe = root.join("bin").join("OmarchyOverlay.exe");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        match find(&exe, &root, None) {
            Found::Use(active) => assert!(active.id.is_none()),
            Found::Missing { .. } => panic!("dev folder should stay local"),
        }
        let _ = fs::remove_dir_all(root);
    }
}
