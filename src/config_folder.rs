//! The config folder beside OmarchyOverlay.exe. your own copies of binds.conf,
//! menu.json, webapps.json, always-float.json and themes\*.toml are read from it;
//! config\examples holds the built-in versions and is the only part Omarchy Overlay writes
//! on start. A file with a mistake is skipped, its built-in version is used and the
//! file and line are reported.
use super::*;
use crate::{always_float, binds, menu_model, theme_files, webapps};

pub(super) struct Loaded {
    pub(super) binds: Vec<binds::Bind>,
    pub(super) menus: menu_model::Menus,
    pub(super) webapps: Vec<webapps::WebApp>,
    pub(super) always_float: always_float::List,
    /// One line per file that could not be used, naming the file and line.
    pub(super) errors: Vec<String>,
}

const EXAMPLES: [(&str, &str); 7] = [
    ("README.txt", include_str!("../config/examples/README.txt")),
    ("binds.conf", binds::DEFAULT),
    ("menu.json", menu_model::DEFAULT),
    ("webapps.json", webapps::DEFAULT),
    ("always-float.json", always_float::EXAMPLE),
    (
        "themes\\gruvbox.toml",
        include_str!("../config/examples/themes/gruvbox.toml"),
    ),
    (
        "themes\\everforest.toml",
        include_str!("../config/examples/themes/everforest.toml"),
    ),
];

pub(super) fn folder() -> PathBuf {
    if let Some(settings) = profile::settings() {
        return settings.join("config");
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("config")))
        .unwrap_or_else(|| PathBuf::from("config"))
}

/// Refreshes config\examples. Only files whose text differs are written, and a
/// read-only folder is not an error: the built-in versions are used either way.
fn write_examples(folder: &Path) {
    for (name, text) in EXAMPLES {
        let path = folder.join("examples").join(name);
        if fs::read_to_string(&path).is_ok_and(|old| old.replace("\r\n", "\n") == text) {
            continue;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, text.replace('\n', "\r\n"));
    }
}

/// Reads one of the user's files. None when there is no copy, which is not an error.
fn read(folder: &Path, name: &str, errors: &mut Vec<String>) -> Option<String> {
    let path = folder.join(name);
    if !path.exists() {
        return None;
    }
    match profile::read_text(&path) {
        Ok(text) => Some(text),
        Err(error) => {
            errors.push(format!("{name}: {error}"));
            None
        }
    }
}

fn themes(folder: &Path, errors: &mut Vec<String>) -> Vec<theme_files::Theme> {
    let Ok(entries) = fs::read_dir(folder.join("themes")) else {
        return vec![];
    };
    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("toml"))
        })
        .collect();
    paths.sort();
    let mut themes: Vec<theme_files::Theme> = vec![];
    for path in paths.into_iter().take(64) {
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = profile::read_text(&path).and_then(|text| theme_files::parse(&id, &text));
        match parsed {
            Ok(theme) if theme::NAMES.iter().any(|(name, _)| *name == theme.id) => errors.push(
                format!("themes\\{id}.toml: {id} is a built-in theme; rename the file."),
            ),
            Ok(theme) => themes.push(theme),
            Err(error) => errors.push(error),
        }
    }
    themes
}

/// Reads every config file. Each one falls back to its built-in version on its own.
pub(super) fn load() -> Loaded {
    let folder = folder();
    write_examples(&folder);
    let mut errors = vec![];
    let binds = read(&folder, "binds.conf", &mut errors)
        .and_then(|text| binds::parse(&text).map_err(|e| errors.push(e)).ok())
        .unwrap_or_else(binds::defaults);
    let menus = read(&folder, "menu.json", &mut errors)
        .and_then(|text| menu_model::parse(&text).map_err(|e| errors.push(e)).ok())
        .unwrap_or_else(menu_model::defaults);
    let webapps = read(&folder, "webapps.json", &mut errors)
        .and_then(|text| webapps::parse(&text).map_err(|e| errors.push(e)).ok())
        .unwrap_or_else(webapps::defaults);
    let always_float = read(&folder, "always-float.json", &mut errors)
        .and_then(|text| always_float::parse(&text).map_err(|e| errors.push(e)).ok())
        .unwrap_or_default();
    theme_files::install(themes(&folder, &mut errors));
    for error in &errors {
        log(&format!(
            "Config file skipped, using the built-in version. {error}"
        ));
    }
    Loaded {
        binds,
        menus,
        webapps,
        always_float,
        errors,
    }
}

/// Writes one of the user's config files after a change made in the menu.
pub(super) fn save(name: &str, text: &str) -> Result<(), String> {
    let folder = folder();
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(name);
    let temporary = folder.join(format!("{name}.tmp"));
    fs::write(&temporary, text.replace("\r\n", "\n").replace('\n', "\r\n"))
        .map_err(|e| format!("{name} could not be saved: {e}"))?;
    fs::rename(&temporary, &path).map_err(|e| format!("{name} could not be saved: {e}"))
}
