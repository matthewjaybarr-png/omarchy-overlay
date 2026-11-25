//! Themes VS Code, VS Code Insiders, VSCodium and Cursor, for editors this user has run
//! (their settings folder exists). Each gets a local "Omarchy Overlay" theme extension
//! and `workbench.colorTheme` set to it. The first original value is journaled and put
//! back by Remove, on exit and after a crash, unless the user picked another theme since.
use super::*;
use crate::vscode_theme as vs;

const RECORD: &str = "vscode-appearance-restore.json";

/// (settings folder under %APPDATA%, extensions folder under the user profile).
const EDITORS: &[(&str, &str)] = &[
    ("Code", ".vscode"),
    ("Code - Insiders", ".vscode-insiders"),
    ("VSCodium", ".vscode-oss"),
    ("Cursor", ".cursor"),
];

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    settings: PathBuf,
    /// The color theme settings.json named before, None when it named none.
    before: Option<String>,
    extensions: PathBuf,
}

fn editors() -> Vec<(PathBuf, PathBuf)> {
    let (Some(appdata), Some(profile)) = (std::env::var_os("APPDATA"), std::env::var_os("USERPROFILE")) else {
        return vec![];
    };
    EDITORS
        .iter()
        .map(|(settings, extensions)| {
            (
                PathBuf::from(&appdata).join(settings).join("User"),
                PathBuf::from(&profile).join(extensions).join("extensions"),
            )
        })
        .filter(|(user, _)| user.is_dir())
        .collect()
}
fn read_record(directory: &Path) -> Result<Vec<Entry>, String> {
    let path = directory.join(RECORD);
    if !path.exists() {
        return Ok(vec![]);
    }
    let entries: Vec<Entry> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| format!("{RECORD}: {e}"))?;
    if entries.len() > EDITORS.len() {
        return Err(format!("{RECORD} is not valid."));
    }
    Ok(entries)
}
fn read(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}
fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension(format!("omarchy-{}.tmp", std::process::id()));
    fs::write(&temporary, text).map_err(|e| e.to_string())?;
    fs::rename(&temporary, path).map_err(|e| {
        let _ = fs::remove_file(&temporary);
        format!("{}: {e}", path.display())
    })
}
/// True when the themed VS Code setup is in place.
pub(super) fn applied(directory: &Path) -> bool {
    directory.join(RECORD).exists()
}
/// The names of the editors found on this PC, for the page.
pub(super) fn found() -> Vec<&'static str> {
    let names = ["VS Code", "VS Code Insiders", "VSCodium", "Cursor"];
    let found = editors();
    EDITORS
        .iter()
        .zip(names)
        .filter(|((settings, _), _)| found.iter().any(|(user, _)| user.parent().is_some_and(|p| p.ends_with(settings))))
        .map(|(_, name)| name)
        .collect()
}
/// Installs the theme's colors as the "Omarchy Overlay" VS Code theme and selects it.
pub(super) fn apply(directory: &Path, theme_name: &str) -> Result<usize, String> {
    let theme = crate::theme_files::resolve(theme_name).ok_or("Unknown theme.")?;
    let found = editors();
    if found.is_empty() {
        return Err("No VS Code, VS Code Insiders, VSCodium or Cursor settings were found for this user.".into());
    }
    let json = vs::render(&theme.colors, theme.dark);
    let (version, package) = vs::package(&json, theme.dark);
    let mut record = read_record(directory)?;
    for (user, extensions) in &found {
        let settings = user.join("settings.json");
        let text = read(&settings)?;
        // Journal first, once per editor: the value from before Omarchy Overlay.
        if !record.iter().any(|e| e.settings == settings) {
            record.push(Entry {
                settings: settings.clone(),
                before: vs::color_theme(&text).filter(|t| t != vs::THEME_LABEL),
                extensions: extensions.clone(),
            });
            write_json_atomic(&directory.join(RECORD), &record)?;
        }
        let folder = extensions.join(vs::EXTENSION_NAME);
        write(&folder.join("themes").join("omarchy-overlay-color-theme.json"), &json)?;
        write(
            &folder.join("package.json"),
            &serde_json::to_string_pretty(&package).map_err(|e| e.to_string())?,
        )?;
        let list = extensions.join("extensions.json");
        let registered = vs::register(&read(&list)?, Some((&version, &folder.to_string_lossy())))?;
        write(&list, &registered)?;
        if vs::color_theme(&text).as_deref() != Some(vs::THEME_LABEL) {
            write(&settings, &vs::set_color_theme(&text, Some(vs::THEME_LABEL)))?;
        }
    }
    Ok(found.len())
}
/// Puts back each editor's color theme and removes the theme extension.
pub(super) fn restore(directory: &Path) -> Result<(), String> {
    let record = read_record(directory)?;
    let mut errors = vec![];
    for entry in &record {
        let result = (|| {
            let text = read(&entry.settings)?;
            // Only our own choice is put back; a theme the user picked since stays.
            if vs::color_theme(&text).as_deref() == Some(vs::THEME_LABEL) {
                write(&entry.settings, &vs::set_color_theme(&text, entry.before.as_deref()))?;
            }
            let list = entry.extensions.join("extensions.json");
            if list.exists() {
                write(&list, &vs::register(&read(&list)?, None)?)?;
            }
            let folder = entry.extensions.join(vs::EXTENSION_NAME);
            if folder.exists() {
                fs::remove_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
            }
            Ok::<(), String>(())
        })();
        if let Err(e) = result {
            errors.push(e);
        }
    }
    if !errors.is_empty() {
        return Err(format!("VS Code theme: {}", errors.join("; ")));
    }
    let path = directory.join(RECORD);
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
