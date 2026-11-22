//! User-owned launcher entries. Relative paths follow the active profile folder.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub target: String,
    pub arguments: String,
    pub working_directory: String,
}
fn valid_path(text: &str) -> bool {
    if text.len() > 4096 || text.contains('"') || text.chars().any(char::is_control) {
        return false;
    }
    if text.contains(':') {
        let b = text.as_bytes();
        return b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && matches!(b[2], b'\\' | b'/')
            && !text[2..].contains(':');
    }
    !text.starts_with('/') && (!text.starts_with('\\') || text.starts_with("\\\\"))
}
impl Application {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > 80
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("Invalid application identifier.".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 120
            || self.name.chars().any(char::is_control)
        {
            return Err("Enter an application name (up to 120 characters).".into());
        }
        if self.target.trim().is_empty()
            || !valid_path(&self.target)
            || !valid_path(&self.working_directory)
        {
            return Err("Use a relative file/folder path or a full Windows drive/UNC path, without quotes or control characters.".into());
        }
        if self.arguments.len() > 4096 || self.arguments.contains(['\0', '\r', '\n']) {
            return Err("Arguments must be one line, up to 4096 characters.".into());
        }
        Ok(())
    }
    pub fn key(&self) -> String {
        format!("custom:{}", self.id)
    }
    pub fn target_path(&self, profile_folder: &Path) -> PathBuf {
        resolve(profile_folder, &self.target)
    }
    pub fn working_path(&self, profile_folder: &Path) -> Option<PathBuf> {
        if self.working_directory.is_empty() {
            let target = self.target_path(profile_folder);
            if target.is_dir() {
                Some(target)
            } else {
                target.parent().map(Path::to_path_buf)
            }
        } else {
            Some(resolve(profile_folder, &self.working_directory))
        }
    }
}
/// 0.18: a path on the same drive as the settings folder is stored relative to it, so the
/// profile keeps working when the stick gets another drive letter. Other paths are unchanged.
pub fn relative_to_settings(settings: &Path, text: &str) -> String {
    let base = settings.to_string_lossy();
    let drive = |t: &str| {
        let b = t.as_bytes();
        (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'\\' | b'/'))
            .then(|| b[0].to_ascii_uppercase())
    };
    let (Some(from), Some(to)) = (drive(&base), drive(text)) else {
        return text.to_string();
    };
    if from != to {
        return text.to_string();
    }
    let parts = |t: &str| {
        t[3..]
            .split(['\\', '/'])
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let (base, target) = (parts(&base), parts(text));
    let common = base
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();
    let mut out = vec!["..".to_string(); base.len() - common];
    out.extend(target[common..].iter().cloned());
    if out.is_empty() {
        ".".into()
    } else {
        out.join("\\")
    }
}
fn resolve(base: &Path, text: &str) -> PathBuf {
    if text.as_bytes().get(1) == Some(&b':') || text.starts_with("\\\\") {
        PathBuf::from(text)
    } else {
        base.join(text)
    }
}
pub fn validate(apps: &[Application]) -> Result<(), String> {
    if apps.len() > 128 {
        return Err("Up to 128 custom applications are supported.".into());
    }
    for (index, app) in apps.iter().enumerate() {
        app.validate()?;
        if apps[..index].iter().any(|old| old.id == app.id) {
            return Err("Application identifiers must be unique.".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_paths_and_favorite_identity_survive_profile_moves() {
        let app = Application {
            id: "editor".into(),
            name: "My editor".into(),
            target: "tools/editor.exe".into(),
            arguments: "--file \"a b.txt\"".into(),
            ..Default::default()
        };
        app.validate().unwrap();
        assert_eq!(
            app.target_path(Path::new("one")),
            Path::new("one/tools/editor.exe")
        );
        assert_eq!(
            app.target_path(Path::new("two")),
            Path::new("two/tools/editor.exe")
        );
        assert_eq!(app.key(), "custom:editor");
        assert_eq!(
            app.working_path(Path::new("one")),
            Some(PathBuf::from("one/tools"))
        );
        let full = Application {
            target: "D:\\Tools\\editor.exe".into(),
            ..app.clone()
        };
        full.validate().unwrap();
        assert_eq!(
            full.target_path(Path::new("elsewhere")).to_string_lossy(),
            full.target
        );
        let mut history = crate::picker::History::default();
        history.toggle(&app.key());
        assert!(history.favorite(&full.key()));
    }
    #[test]
    fn paths_on_the_settings_drive_become_relative() {
        let settings = Path::new("G:\\Omarchy Overlay\\OVERLAY SETTINGS");
        assert_eq!(
            relative_to_settings(settings, "g:\\PortableApps\\Editor\\editor.exe"),
            "..\\..\\PortableApps\\Editor\\editor.exe"
        );
        assert_eq!(
            relative_to_settings(settings, "G:\\Omarchy Overlay\\OVERLAY SETTINGS\\wallpapers\\a.png"),
            "wallpapers\\a.png"
        );
        assert_eq!(relative_to_settings(settings, "C:\\Windows\\notepad.exe"), "C:\\Windows\\notepad.exe");
        assert_eq!(relative_to_settings(settings, "tools\\x.exe"), "tools\\x.exe");
        assert_eq!(relative_to_settings(Path::new("relative"), "G:\\x.exe"), "G:\\x.exe");
        let app = Application {
            id: "a".into(),
            name: "a".into(),
            target: relative_to_settings(settings, "G:\\PortableApps\\x.exe"),
            ..Default::default()
        };
        app.validate().unwrap();
    }
    #[test]
    fn rejects_ambiguous_paths_duplicate_ids_and_malformed_entries() {
        let app = Application {
            id: "test".into(),
            name: "Test".into(),
            target: "tool.exe".into(),
            ..Default::default()
        };
        for target in [
            "C:tool.exe",
            "\\tool.exe",
            "https://example.com",
            "\"tool.exe\"",
            "bad\nfile",
        ] {
            assert!(Application {
                target: target.into(),
                ..app.clone()
            }
            .validate()
            .is_err());
        }
        assert!(validate(&[app.clone(), app]).is_err());
    }
}
