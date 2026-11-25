//! Obsidian themed as omarchy-theme-set-obsidian does: every vault Obsidian knows gets an
//! "Omarchy Overlay" theme (`.obsidian\themes\Omarchy Overlay`) in the current theme's
//! colors, which the user picks once in Obsidian's Appearance settings. Obsidian's own
//! settings are never edited. The folders it adds are journaled and removed again by
//! Remove, on exit and after a crash; the next start adds them back while it is on.
use super::*;

const RECORD: &str = "obsidian-appearance-restore.json";
const NAME: &str = "Omarchy Overlay";
const MANIFEST: &str = r#"{
  "name": "Omarchy Overlay",
  "version": "1.0.0",
  "minAppVersion": "0.16.0",
  "description": "Follows the Omarchy Overlay theme's colors",
  "author": "Omarchy Overlay"
}
"#;

/// Vault folders listed in %APPDATA%\obsidian\obsidian.json that exist.
fn vaults() -> Vec<PathBuf> {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return vec![];
    };
    let Ok(text) = fs::read_to_string(PathBuf::from(appdata).join("obsidian").join("obsidian.json")) else {
        return vec![];
    };
    let Ok(list) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    list["vaults"]
        .as_object()
        .map(|v| {
            v.values()
                .filter_map(|vault| vault["path"].as_str().map(PathBuf::from))
                .filter(|path| path.join(".obsidian").is_dir())
                .take(64)
                .collect()
        })
        .unwrap_or_default()
}
fn record(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        return Ok(vec![]);
    }
    let folders: Vec<PathBuf> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| format!("{RECORD}: {e}"))?;
    // Only theme folders of this name are ever removed.
    if folders.iter().any(|f| !f.ends_with(Path::new(".obsidian").join("themes").join(NAME))) {
        return Err(format!("{RECORD} is not valid; it was kept."));
    }
    Ok(folders)
}
/// How many vaults Obsidian lists, for the page.
pub(super) fn vault_count() -> usize {
    vaults().len()
}
/// Writes the theme into every vault. Returns how many vaults got it.
pub(super) fn apply(directory: &Path, theme_name: &str) -> Result<usize, String> {
    let theme = crate::theme_files::resolve(theme_name).ok_or("Unknown theme.")?;
    let vaults = vaults();
    if vaults.is_empty() {
        return Err("Obsidian lists no vaults for this user.".into());
    }
    let css = crate::vscode_theme::obsidian_css(&theme.colors, theme.dark);
    let mut folders = record(directory)?;
    for vault in &vaults {
        let folder = vault.join(".obsidian").join("themes").join(NAME);
        // Journaled before it exists, so a crash between the two still removes it.
        if !folder.exists() && !folders.contains(&folder) {
            folders.push(folder.clone());
            write_json_atomic(&directory.join(RECORD), &folders)?;
        }
        if !folders.contains(&folder) {
            continue; // A folder of this name the user made themselves: left alone.
        }
        fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        fs::write(folder.join("manifest.json"), MANIFEST).map_err(|e| e.to_string())?;
        fs::write(folder.join("theme.css"), &css).map_err(|e| e.to_string())?;
    }
    Ok(vaults.len())
}
/// Removes the theme folders Omarchy Overlay added.
pub(super) fn restore(directory: &Path) -> Result<(), String> {
    let folders = record(directory)?;
    for folder in folders {
        if folder.exists() {
            fs::remove_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        }
    }
    let path = directory.join(RECORD);
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
