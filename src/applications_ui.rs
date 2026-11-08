use super::*;
use windows_sys::Win32::UI::Controls::Dialogs::*;
/// The Windows file picker for a custom app (0.17: opened from the menu).
pub(super) unsafe fn choose(owner: HWND) -> Result<Option<PathBuf>, String> {
    let mut buffer = vec![0u16; 32768];
    let filter = wide("Applications and shortcuts\0*.exe;*.lnk;*.cmd;*.bat\0All files\0*.*\0");
    let title = wide("Choose application, file or shortcut");
    let mut dialog: OPENFILENAMEW = zeroed();
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = owner;
    dialog.lpstrFile = buffer.as_mut_ptr();
    dialog.nMaxFile = buffer.len() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.Flags = OFN_FILEMUSTEXIST | OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_DONTADDTORECENT;
    if GetOpenFileNameW(&mut dialog) != 0 {
        let count = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Ok(Some(PathBuf::from(String::from_utf16_lossy(
            &buffer[..count],
        ))))
    } else if CommDlgExtendedError() == 0 {
        Ok(None)
    } else {
        Err("Windows could not open the file picker.".into())
    }
}
impl App {
    pub(super) fn profile_folder(&self) -> &Path {
        self.profile.path.parent().unwrap_or(Path::new("."))
    }
    /// 0.17: Custom apps is a page of the Omarchy Overlay menu.
    pub(super) unsafe fn show_applications(&mut self) {
        ShowWindow(self.launcher, SW_HIDE);
        self.open_level(overlay::Level::Page("custom-apps"), true);
    }
    pub(super) unsafe fn refresh_apps(&mut self) {
        let mut entries = collect_apps();
        entries.extend(self.config.applications.iter().map(|app| Entry {
            label: app.name.clone(),
            path: app.target_path(self.profile_folder()),
            arguments: app.arguments.clone(),
            custom_key: Some(app.key()),
            working_directory: app.working_path(self.profile_folder()),
            action: None,
        }));
        if self.command_menu {
            entries.extend(command_menu::entries());
        }
        entries.sort_by_key(|entry| entry.label.to_lowercase());
        self.entries = entries;
        self.picker_icons.clear();
        self.filter_apps();
    }
}
