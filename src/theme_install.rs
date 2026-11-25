//! Omarchy's omarchy-theme-install: a theme from its git repository becomes
//! config\themes\<id>. Only colors.toml and the background pictures are kept; like
//! Omarchy for an installed theme, nothing that runs code (Lua, terminal configs,
//! editor extensions) is copied. Needs git on the PC; the clone runs off the UI thread.
use super::*;

/// Posted to the main window when an install ends; lParam is a Box<Result<id, error>>.
pub(super) const DONE: u32 = WM_APP + 78;
/// Pictures Windows can show as a wallpaper.
const PICTURES: [&str; 4] = ["jpg", "jpeg", "png", "bmp"];

fn install(url: &str, id: &str, themes: &Path) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let temp = std::env::temp_dir().join(format!("omarchy-overlay-theme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp);
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--quiet", "--", url])
        .arg(&temp)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|_| "Installing a theme needs git. Install Git for Windows (winget install Git.Git) and try again.".to_string())?;
    let result = (|| {
        if !status.success() {
            return Err(format!("git could not download {url}."));
        }
        let colors = fs::read_to_string(temp.join("colors.toml"))
            .map_err(|_| "That repository has no colors.toml, so it is not an Omarchy theme.".to_string())?;
        crate::theme_files::parse(id, &colors)?;
        let target = themes.join(id);
        let _ = fs::remove_dir_all(&target);
        fs::create_dir_all(target.join("backgrounds")).map_err(|e| e.to_string())?;
        fs::write(target.join("colors.toml"), &colors).map_err(|e| e.to_string())?;
        let mut pictures = 0;
        if let Ok(entries) = fs::read_dir(temp.join("backgrounds")) {
            for entry in entries.flatten() {
                let path = entry.path();
                let kind = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                if path.is_file() && PICTURES.contains(&kind.as_str()) && pictures < 64 {
                    if let Some(name) = path.file_name() {
                        fs::copy(&path, target.join("backgrounds").join(name)).map_err(|e| e.to_string())?;
                        pictures += 1;
                    }
                }
            }
        }
        Ok(id.to_string())
    })();
    let _ = fs::remove_dir_all(&temp);
    result
}
impl App {
    /// Starts installing a theme; the main window hears back through DONE.
    pub(super) unsafe fn install_theme(&mut self, text: &str) {
        let Some((url, id)) = crate::theme_files::theme_repo(text) else {
            return;
        };
        let themes = config_folder::folder().join("themes");
        let owner = self.main as isize;
        self.show_feedback(&format!("Installing the {id} theme…"));
        std::thread::spawn(move || {
            let result = Box::new(install(&url, &id, &themes));
            PostMessageW(owner as HWND, DONE, 0, Box::into_raw(result) as isize);
        });
    }
    /// The install ended: load the new theme and switch to it.
    pub(super) unsafe fn theme_installed(&mut self, result: *mut Result<String, String>) {
        let result = *Box::from_raw(result);
        match result {
            Ok(id) => {
                self.reload_config();
                self.apply_theme(&id);
                self.show_feedback(&format!("Theme {id} installed"));
            }
            Err(error) => {
                log(&format!("Theme install: {error}"));
                message(&error);
            }
        }
    }
}
