//! The selection and link colors of classic Windows controls (list boxes, edit boxes,
//! menus, older dialogs) follow the theme accent, the nearest Windows match to the GTK
//! accent Omarchy sets. Only those three system colors change: window and text colors
//! stay, because many older apps hard-code dark text on the window color.
//! SetSysColors lasts until sign-out, so the originals are journaled and put back on
//! Restore, on exit and after a crash.
use super::*;

const RECORD: &str = "windows-syscolors-restore.json";
const COLORS: [i32; 3] = [
    COLOR_HIGHLIGHT,
    COLOR_HIGHLIGHTTEXT,
    COLOR_HOTLIGHT,
];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    before: Vec<(i32, u32)>,
}

unsafe fn set(pairs: &[(i32, u32)]) -> Result<(), String> {
    let indexes: Vec<i32> = pairs.iter().map(|p| p.0).collect();
    let colors: Vec<u32> = pairs.iter().map(|p| p.1).collect();
    if SetSysColors(pairs.len() as i32, indexes.as_ptr(), colors.as_ptr()) == 0 {
        return Err("Windows did not allow the system color change.".into());
    }
    Ok(())
}
/// Sets the selection and link colors from the accent, journaling the originals first.
pub(super) unsafe fn apply(directory: &Path, accent: u32) -> Result<(), String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        let before = COLORS.iter().map(|&i| (i, GetSysColor(i))).collect();
        write_json_atomic(&path, &Record { before })?;
    }
    let next: Vec<(i32, u32)> = COLORS.into_iter().zip(crate::theme_files::syscolor_values(accent)).collect();
    set(&next)
}
pub(super) unsafe fn restore(directory: &Path) -> Result<(), String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        return Ok(());
    }
    let record: Record =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| format!("{RECORD}: {e}"))?;
    if record.before.len() != COLORS.len() || record.before.iter().any(|(i, _)| !COLORS.contains(i)) {
        return Err(format!("{RECORD} is not valid; it was kept."));
    }
    set(&record.before)?;
    fs::remove_file(path).map_err(|e| e.to_string())
}

