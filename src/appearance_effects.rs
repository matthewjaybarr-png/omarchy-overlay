//! Explicit, reversible user-level wallpaper and Terminal fragment integration.
use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    before: Option<String>,
    acceptable: Vec<String>,
}
fn terminal_path(directory: &Path) -> PathBuf {
    directory
        .parent()
        .unwrap_or(directory)
        .join("Microsoft/Windows Terminal/Fragments/OmarchyOverlay/appearance.json")
}
unsafe fn wallpaper() -> Result<String, String> {
    let mut buffer = vec![0u16; 32768];
    if SystemParametersInfoW(
        SPI_GETDESKWALLPAPER,
        buffer.len() as u32,
        buffer.as_mut_ptr().cast(),
        0,
    ) == 0
    {
        return Err("Windows could not read the current wallpaper.".into());
    }
    let n = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..n]))
}
unsafe fn set_wallpaper(path: &str) -> Result<(), String> {
    let mut path = wide(path);
    if SystemParametersInfoW(
        SPI_SETDESKWALLPAPER,
        0,
        path.as_mut_ptr().cast(),
        SPIF_SENDCHANGE,
    ) == 0
    {
        return Err("Windows did not allow the wallpaper change.".into());
    }
    Ok(())
}
fn read_journal(path: &Path) -> Result<Journal, String> {
    let j: Journal = serde_json::from_str(&profile::read_text(path)?).map_err(|e| e.to_string())?;
    if j.acceptable.len() > 2
        || j.before.as_ref().is_some_and(|s| s.len() > 1024 * 1024)
        || j.acceptable.iter().any(|s| s.len() > 1024 * 1024)
    {
        return Err("Invalid appearance recovery record.".into());
    }
    Ok(j)
}
pub(super) unsafe fn apply_wallpaper(directory: &Path, path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("Choose an existing wallpaper image on this PC.".into());
    }
    let current = wallpaper()?;
    let record = directory.join("wallpaper-restore.json");
    let old = if record.exists() {
        read_journal(&record)?
    } else {
        Journal {
            before: Some(current.clone()),
            acceptable: vec![],
        }
    };
    if !old.acceptable.is_empty()
        && !old
            .acceptable
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&current))
    {
        return Err(
            "Wallpaper changed outside Omarchy Overlay. Restore or review the previous change first."
                .into(),
        );
    }
    let next = path.to_string_lossy().to_string();
    write_json_atomic(
        &record,
        &Journal {
            before: old.before,
            acceptable: vec![current, next.clone()],
        },
    )?;
    set_wallpaper(&next)
}
fn color(value: u32) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        value & 255,
        (value >> 8) & 255,
        (value >> 16) & 255
    )
}
pub(super) fn apply_terminal(directory: &Path, theme_name: &str) -> Result<(), String> {
    let path = terminal_path(directory);
    let record = directory.join("terminal-appearance-restore.json");
    let current = if path.exists() {
        Some(profile::read_text(&path)?)
    } else {
        None
    };
    let old = if record.exists() {
        read_journal(&record)?
    } else {
        Journal {
            before: current.clone(),
            acceptable: vec![],
        }
    };
    if !old.acceptable.is_empty()
        && current
            .as_ref()
            .is_some_and(|s| !old.acceptable.contains(s))
    {
        return Err("The Terminal fragment changed outside Omarchy Overlay; it was left intact.".into());
    }
    let palette = theme::palette(theme_name).ok_or("Unknown theme.")?;
    let fragment = crate::appearance_profiles::terminal_fragment(
        theme_name,
        &color(palette.background),
        &color(palette.text),
        &color(palette.accent),
    );
    let next = serde_json::to_string_pretty(&fragment).map_err(|e| e.to_string())?;
    let mut acceptable = current.into_iter().collect::<Vec<_>>();
    acceptable.push(next);
    write_json_atomic(
        &record,
        &Journal {
            before: old.before,
            acceptable,
        },
    )?;
    fs::create_dir_all(
        path.parent()
            .ok_or("Terminal fragment folder is unavailable.")?,
    )
    .map_err(|e| e.to_string())?;
    write_json_atomic(&path, &fragment)
}
/// Puts back the wallpaper Windows had before Omarchy Overlay first changed it.
pub(super) unsafe fn restore_wallpaper(directory: &Path) -> Result<(), String> {
    let record = directory.join("wallpaper-restore.json");
    if !record.exists() {
        return Ok(());
    }
    let j = read_journal(&record)?;
    let current = wallpaper()?;
    let before = j
        .before
        .ok_or("Wallpaper recovery lacks an original value.")?;
    if current.eq_ignore_ascii_case(&before)
        || j.acceptable
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&current))
    {
        set_wallpaper(&before)?;
        fs::remove_file(&record).map_err(|e| e.to_string())
    } else {
        Err("Wallpaper was changed externally; recovery was preserved without replacing it.".into())
    }
}
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let mut errors = vec![];
    if let Err(e) = windows_personalization::restore(directory) {
        errors.push(e)
    }
    if let Err(error) = restore_wallpaper(directory) {
        errors.push(error);
    }
    if let Err(error) = restore_terminal(directory) {
        errors.push(error);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\r\n"))
    }
}
/// True when Omarchy Overlay's themed Windows Terminal profile is in place.
pub(super) fn terminal_applied(directory: &Path) -> bool {
    terminal_path(directory).exists()
}
/// Puts back only the Windows Terminal fragment (0.17: the Terminal colors page).
pub(super) fn restore_terminal(directory: &Path) -> Result<(), String> {
    let record = directory.join("terminal-appearance-restore.json");
    if !record.exists() {
        return Ok(());
    }
    let j = read_journal(&record)?;
    let path = terminal_path(directory);
    let current = if path.exists() {
        Some(profile::read_text(&path)?)
    } else {
        None
    };
    if current != j.before && !current.as_ref().is_some_and(|s| j.acceptable.contains(s)) {
        return Err("Terminal fragment was changed externally; its recovery was preserved.".into());
    }
    if let Some(before) = j.before {
        write_text_atomic(&path, &before)?;
    } else if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    fs::remove_file(&record).map_err(|e| e.to_string())
}
fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    // Retain any preexisting fragment's exact text instead of parsing/reformatting it.
    use std::io::Write;
    let temporary = path.with_extension(format!("restore-{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e: std::io::Error| e.to_string())
}
