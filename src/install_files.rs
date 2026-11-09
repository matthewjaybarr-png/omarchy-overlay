//! The Admin install's decisions and file handling, kept free of Windows calls so they
//! are tested on every build host: what an install will do (fresh, update, repair,
//! downgrade), and replacing the program files so a failure never leaves a broken mix.
//!
//! Files are replaced in two steps. Every new file is first copied beside its target as
//! `<name>.install-new` and checked byte for byte; nothing in use changes until all of
//! them are ready. Then each target is renamed aside to `<name>.install-old` and the new
//! file renamed into place. Windows lets a running program's file be renamed, so an old
//! copy that is still running cannot block an update. If any step fails, the files
//! already swapped are put back. A manifest lists what was installed, so files a newer
//! release no longer ships are removed, and nothing else in the folder is touched.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const EXE: &str = "OmarchyOverlay.exe";
pub const MANIFEST: &str = "install-manifest.txt";
/// Files beside the exe in a release that the install copies when present.
const EXTRAS: [&str; 3] = ["RECOVER.cmd", "README.txt", "LICENSE.txt"];
const NEW: &str = ".install-new";
const OLD: &str = ".install-old";

/// What installing this version over what is there will do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    Fresh,
    Update { from: String },
    /// The same version again: the files and registrations are put back as new.
    Reinstall,
    /// A copy is there but its version is unknown, for example after a failed uninstall.
    Repair,
    Downgrade { from: String },
}

/// `0.2.0`, `v0.2` or `0.3.0-beta` as numbers; None when it is not a version.
pub fn parse_version(text: &str) -> Option<[u32; 3]> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let core = text.split(['-', '+', ' ']).next()?;
    let mut parts = [0u32; 3];
    let mut count = 0;
    for (i, part) in core.split('.').enumerate() {
        if i >= 3 {
            return None;
        }
        parts[i] = part.parse().ok()?;
        count += 1;
    }
    (count > 0).then_some(parts)
}

/// `installed` is the version in the Apps entry, `files_present` whether the install
/// folder has the program in it.
pub fn plan(installed: Option<&str>, files_present: bool, this: &str) -> Plan {
    let Some(from) = installed else {
        return if files_present { Plan::Repair } else { Plan::Fresh };
    };
    match (parse_version(from), parse_version(this)) {
        (Some(old), Some(new)) if old < new => Plan::Update { from: from.trim().into() },
        (Some(old), Some(new)) if old > new => Plan::Downgrade { from: from.trim().into() },
        (Some(_), Some(_)) => Plan::Reinstall,
        _ => Plan::Repair,
    }
}

/// The tray item that starts an install, or None for a copy that is the installed one.
pub fn tray_label(installed: Option<&str>, this: &str) -> String {
    match plan(installed, false, this) {
        Plan::Fresh => "Install (admin)…".into(),
        Plan::Update { .. } => format!("Update the installed copy to {this} (admin)…"),
        Plan::Downgrade { from } => format!("Replace installed {from} with older {this} (admin)…"),
        Plan::Reinstall | Plan::Repair => "Repair the installed copy (admin)…".into(),
    }
}

/// The question asked before a downgrade; None when the install needs no question.
pub fn question(plan: &Plan, this: &str) -> Option<String> {
    match plan {
        Plan::Downgrade { from } => Some(format!(
            "Omarchy Overlay {from} is installed, which is newer than this copy ({this}).\r\n\r\nReplace it with {this}? Your settings are kept."
        )),
        _ => None,
    }
}

pub fn success_message(plan: &Plan, this: &str, dir: &Path, user: &str, pending: bool) -> String {
    let what = match plan {
        Plan::Fresh => String::new(),
        Plan::Update { from } => format!(": updated from {from} to {this}"),
        Plan::Downgrade { from } => format!(": replaced {from} with {this}"),
        Plan::Reinstall => format!(": {this} reinstalled"),
        Plan::Repair => format!(": repaired with {this}"),
    };
    let mut text = format!(
        "Omarchy Overlay is installed in {}{what}. It starts when {user} signs in.\r\nRemove it from Settings > Apps or the tray menu.",
        dir.display()
    );
    if pending {
        text.push_str("\r\n\r\nSome old files were still in use. Windows removes them at the next restart.");
    }
    text
}

/// One file the install puts in place: where it comes from and its path in the install
/// folder, with `/` between folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub source: PathBuf,
    pub relative: String,
}

/// Everything the install copies from the unzipped release that holds `exe`.
pub fn release_files(exe: &Path) -> Vec<Item> {
    let folder = exe.parent().unwrap_or(Path::new("."));
    let mut items = vec![Item {
        source: exe.to_path_buf(),
        relative: EXE.into(),
    }];
    for name in EXTRAS {
        let path = folder.join(name);
        if path.is_file() {
            items.push(Item {
                source: path,
                relative: name.into(),
            });
        }
    }
    let mut fonts: Vec<PathBuf> = fs::read_dir(folder.join("fonts"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    fonts.sort();
    for font in fonts {
        if let Some(name) = font.file_name().and_then(|n| n.to_str()) {
            items.push(Item {
                relative: format!("fonts/{name}"),
                source: font.clone(),
            });
        }
    }
    items
}

/// A manifest path is used only when it stays inside the install folder.
fn safe_relative(relative: &str) -> bool {
    !relative.is_empty()
        && relative
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != ".." && !p.contains(['\\', ':']))
}
fn target(dir: &Path, relative: &str) -> PathBuf {
    relative.split('/').fold(dir.to_path_buf(), |p, part| p.join(part))
}
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
/// The files the last install put in `dir`.
pub fn read_manifest(dir: &Path) -> Vec<String> {
    fs::read_to_string(dir.join(MANIFEST))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("version="))
        .filter(|l| safe_relative(l))
        .map(String::from)
        .collect()
}
/// The version the last install wrote into its manifest.
pub fn manifest_version(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join(MANIFEST))
        .ok()?
        .lines()
        .find_map(|l| l.trim().strip_prefix("version=").map(String::from))
}
fn write_manifest(dir: &Path, version: &str, items: &[Item]) -> Result<(), String> {
    let mut text = format!("# Files installed by Omarchy Overlay. Do not edit.\r\nversion={version}\r\n");
    for item in items {
        text.push_str(&item.relative);
        text.push_str("\r\n");
    }
    let temporary = dir.join(format!("{MANIFEST}{NEW}"));
    fs::write(&temporary, text).map_err(|e| format!("Could not write the install manifest: {e}"))?;
    fs::rename(&temporary, dir.join(MANIFEST))
        .map_err(|e| format!("Could not write the install manifest: {e}"))
}
/// True when `a` and `b` hold the same bytes.
fn same_content(a: &Path, b: &Path) -> bool {
    match (fs::metadata(a), fs::metadata(b)) {
        (Ok(x), Ok(y)) if x.len() == y.len() => {
            matches!((fs::read(a), fs::read(b)), (Ok(x), Ok(y)) if x == y)
        }
        _ => false,
    }
}
/// A free `<name>.install-old` name, replacing an earlier leftover when it can.
fn aside_name(path: &Path) -> PathBuf {
    let first = with_suffix(path, OLD);
    if !first.exists() || fs::remove_file(&first).is_ok() {
        return first;
    }
    (2..1000)
        .map(|n| with_suffix(path, &format!("{OLD}-{n}")))
        .find(|p| !p.exists() || fs::remove_file(p).is_ok())
        .unwrap_or(first)
}
/// Removes `.install-new` and `.install-old` files an interrupted run left. Files that are
/// still in use stay and are tried again next time.
pub fn clear_leftovers(dir: &Path) {
    for folder in [dir.to_path_buf(), dir.join("fonts")] {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if (name.ends_with(NEW) || name.contains(OLD)) && entry.path().is_file() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Replaced {
    /// Files an earlier version installed that this one does not ship, now removed.
    pub stale_removed: Vec<String>,
    /// Old files still in use; the caller has Windows delete them at the next restart.
    pub pending_delete: Vec<PathBuf>,
}

/// Puts `items` into `dir` as described at the top of this file.
pub fn replace_files(dir: &Path, items: &[Item], version: &str) -> Result<Replaced, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    clear_leftovers(dir);
    let previous = read_manifest(dir);

    // 1. Stage every file. Nothing the old version uses has changed yet.
    let mut staged: Vec<PathBuf> = vec![];
    let discard = |staged: &[PathBuf]| {
        for path in staged {
            let _ = fs::remove_file(path);
        }
    };
    for item in items {
        let new = with_suffix(&target(dir, &item.relative), NEW);
        let result = (|| {
            if let Some(parent) = new.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&item.source, &new).map_err(|e| e.to_string())?;
            if same_content(&item.source, &new) {
                Ok(())
            } else {
                Err("the copy does not match the original".to_string())
            }
        })();
        if new.exists() {
            staged.push(new);
        }
        if let Err(error) = result {
            discard(&staged);
            return Err(format!("Could not copy {}: {error}", item.source.display()));
        }
    }

    // 2. Swap them in, undoing every swap if one fails.
    let mut swapped: Vec<(PathBuf, Option<PathBuf>)> = vec![];
    let undo = |swapped: &[(PathBuf, Option<PathBuf>)]| {
        for (to, old) in swapped.iter().rev() {
            if fs::remove_file(to).is_err() {
                let _ = fs::rename(to, with_suffix(to, NEW));
            }
            if let Some(old) = old {
                let _ = fs::rename(old, to);
            }
        }
    };
    for (i, item) in items.iter().enumerate() {
        let to = target(dir, &item.relative);
        let new = &staged[i];
        let old = if to.exists() {
            let old = aside_name(&to);
            if let Err(error) = fs::rename(&to, &old) {
                undo(&swapped);
                discard(&staged[i..]);
                return Err(format!("Could not replace {}: {error}", to.display()));
            }
            Some(old)
        } else {
            None
        };
        if let Err(error) = fs::rename(new, &to) {
            if let Some(old) = &old {
                let _ = fs::rename(old, &to);
            }
            undo(&swapped);
            discard(&staged[i..]);
            return Err(format!("Could not put {} in place: {error}", to.display()));
        }
        swapped.push((to, old));
    }

    // 3. Everything is in place: tidy up. Failures here only leave old files behind.
    let mut report = Replaced::default();
    let shipped: HashSet<&str> = items.iter().map(|i| i.relative.as_str()).collect();
    for relative in previous.iter().filter(|r| !shipped.contains(r.as_str())) {
        let path = target(dir, relative);
        if !path.is_file() {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            report.stale_removed.push(relative.clone());
        } else {
            let aside = aside_name(&path);
            if fs::rename(&path, &aside).is_ok() {
                report.pending_delete.push(aside);
            }
        }
    }
    for (_, old) in swapped {
        if let Some(old) = old {
            if fs::remove_file(&old).is_err() {
                report.pending_delete.push(old);
            }
        }
    }
    write_manifest(dir, version, items)?;
    Ok(report)
}

/// The installed size for the Apps entry, in KiB.
pub fn size_kib(items: &[Item]) -> u64 {
    let bytes: u64 = items
        .iter()
        .filter_map(|i| fs::metadata(&i.source).ok())
        .map(|m| m.len())
        .sum();
    bytes.div_ceil(1024)
}

/// True when a running program's image is the install folder's.
pub fn in_folder(image: &Path, dir: &Path) -> bool {
    image
        .parent()
        .is_some_and(|p| p.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "overlay-install-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }
    /// An unzipped release: the exe, RECOVER.cmd and the given fonts.
    fn release(exe: &str, fonts: &[&str]) -> PathBuf {
        let root = temp();
        fs::write(root.join(EXE), exe).unwrap();
        fs::write(root.join("RECOVER.cmd"), "recover").unwrap();
        fs::create_dir_all(root.join("fonts")).unwrap();
        for font in fonts {
            fs::write(root.join("fonts").join(font), format!("font {font}")).unwrap();
        }
        root.join(EXE)
    }
    fn names(dir: &Path) -> Vec<String> {
        let mut found = vec![];
        for folder in [dir.to_path_buf(), dir.join("fonts")] {
            for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
                if entry.path().is_file() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    found.push(if folder == dir { name } else { format!("fonts/{name}") });
                }
            }
        }
        found.sort();
        found
    }

    #[test]
    fn versions_parse_and_order() {
        assert_eq!(parse_version("0.2.0"), Some([0, 2, 0]));
        assert_eq!(parse_version("v1.10"), Some([1, 10, 0]));
        assert_eq!(parse_version(" 0.3.0-beta "), Some([0, 3, 0]));
        assert_eq!(parse_version("next"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
    }
    #[test]
    fn plan_follows_the_installed_version() {
        assert_eq!(plan(None, false, "0.3.0"), Plan::Fresh);
        assert_eq!(plan(None, true, "0.3.0"), Plan::Repair);
        assert_eq!(plan(Some("0.2.0"), true, "0.3.0"), Plan::Update { from: "0.2.0".into() });
        assert_eq!(plan(Some("0.3.0"), true, "0.3.0"), Plan::Reinstall);
        assert_eq!(plan(Some("0.4.0"), true, "0.3.0"), Plan::Downgrade { from: "0.4.0".into() });
        assert_eq!(plan(Some("garbage"), true, "0.3.0"), Plan::Repair);
        assert!(question(&plan(Some("0.4.0"), true, "0.3.0"), "0.3.0").is_some());
        assert!(question(&Plan::Update { from: "0.2.0".into() }, "0.3.0").is_none());
    }
    #[test]
    fn tray_and_messages_name_the_versions() {
        assert_eq!(tray_label(None, "0.3.0"), "Install (admin)…");
        assert!(tray_label(Some("0.2.0"), "0.3.0").starts_with("Update the installed copy to 0.3.0"));
        assert!(tray_label(Some("0.3.0"), "0.3.0").starts_with("Repair"));
        let dir = Path::new(r"C:\Program Files\OmarchyOverlay");
        let fresh = success_message(&Plan::Fresh, "0.3.0", dir, "PC\\me", false);
        // The VM install test reads this prefix.
        assert!(fresh.starts_with("Omarchy Overlay is installed in"), "{fresh}");
        let update = success_message(&Plan::Update { from: "0.2.0".into() }, "0.3.0", dir, "PC\\me", true);
        assert!(update.starts_with("Omarchy Overlay is installed in"));
        assert!(update.contains("updated from 0.2.0 to 0.3.0") && update.contains("next restart"), "{update}");
    }
    #[test]
    fn fresh_install_copies_the_release_and_writes_a_manifest() {
        let exe = release("v1", &["a.ttf", "b.ttf"]);
        let fs_folder = exe.parent().unwrap();
        fs::write(fs_folder.join("LICENSE.txt"), "MIT").unwrap();
        let dir = temp().join("OmarchyOverlay");
        let report = replace_files(&dir, &release_files(&exe), "0.2.0").unwrap();
        assert_eq!(report, Replaced::default());
        assert_eq!(names(&dir), ["LICENSE.txt", EXE, "RECOVER.cmd", "fonts/a.ttf", "fonts/b.ttf", MANIFEST]);
        assert_eq!(fs::read_to_string(dir.join(EXE)).unwrap(), "v1");
        assert_eq!(manifest_version(&dir).as_deref(), Some("0.2.0"));
        assert_eq!(read_manifest(&dir).len(), 5);
    }
    #[test]
    fn update_replaces_files_removes_dropped_ones_and_keeps_others() {
        let dir = temp().join("OmarchyOverlay");
        replace_files(&dir, &release_files(&release("v1", &["a.ttf", "b.ttf"])), "0.2.0").unwrap();
        fs::write(dir.join("notes.txt"), "mine").unwrap();
        let report = replace_files(&dir, &release_files(&release("v2", &["a.ttf", "c.ttf"])), "0.3.0").unwrap();
        assert_eq!(report.stale_removed, ["fonts/b.ttf"]);
        assert!(report.pending_delete.is_empty());
        assert_eq!(names(&dir), [EXE, "RECOVER.cmd", "fonts/a.ttf", "fonts/c.ttf", MANIFEST, "notes.txt"]);
        assert_eq!(fs::read_to_string(dir.join(EXE)).unwrap(), "v2");
        assert_eq!(manifest_version(&dir).as_deref(), Some("0.3.0"));
    }
    #[test]
    fn a_failed_copy_changes_nothing_and_leaves_no_staged_files() {
        let dir = temp().join("OmarchyOverlay");
        replace_files(&dir, &release_files(&release("v1", &["a.ttf"])), "0.2.0").unwrap();
        let before = names(&dir);
        let mut items = release_files(&release("v2", &["a.ttf"]));
        items.push(Item { source: PathBuf::from("/no/such/file.ttf"), relative: "fonts/z.ttf".into() });
        let error = replace_files(&dir, &items, "0.3.0").unwrap_err();
        assert!(error.contains("file.ttf"), "{error}");
        assert_eq!(names(&dir), before);
        assert_eq!(fs::read_to_string(dir.join(EXE)).unwrap(), "v1");
        assert_eq!(manifest_version(&dir).as_deref(), Some("0.2.0"));
    }
    #[test]
    fn leftovers_from_an_interrupted_run_are_cleared() {
        let dir = temp().join("OmarchyOverlay");
        fs::create_dir_all(dir.join("fonts")).unwrap();
        for name in ["OmarchyOverlay.exe.install-new", "OmarchyOverlay.exe.install-old", "fonts/a.ttf.install-old-2"] {
            fs::write(target(&dir, name), "x").unwrap();
        }
        replace_files(&dir, &release_files(&release("v1", &["a.ttf"])), "0.2.0").unwrap();
        assert_eq!(names(&dir), [EXE, "RECOVER.cmd", "fonts/a.ttf", MANIFEST]);
    }
    #[test]
    fn manifest_paths_cannot_leave_the_install_folder() {
        let dir = temp();
        fs::write(dir.join(MANIFEST), "version=0.1.0\r\nfonts/a.ttf\r\n../evil.txt\r\nC:/x\r\nfonts\\b.ttf\r\n").unwrap();
        assert_eq!(read_manifest(&dir), ["fonts/a.ttf"]);
    }
    #[test]
    fn processes_match_the_install_folder_only() {
        let dir = Path::new(r"C:\Program Files\OmarchyOverlay");
        assert!(in_folder(Path::new(r"C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe"), dir) || cfg!(not(windows)));
        assert!(!in_folder(Path::new(r"C:\Program Files\OmarchyOverlay2\OmarchyOverlay.exe"), dir));
        let unix = Path::new("/opt/OmarchyOverlay");
        assert!(in_folder(Path::new("/opt/omarchyoverlay/OmarchyOverlay.exe"), unix));
        assert!(!in_folder(Path::new("/opt/OmarchyOverlay/fonts/x"), unix));
    }
}
