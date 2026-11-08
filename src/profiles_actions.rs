use super::*;
impl App {
    pub(super) unsafe fn show_profiles(&self) {
        set_text(
            GetDlgItem(self.profiles, 90),
            &format!(
                "{} profile\r\n{}",
                if self.profile.id.is_some() {
                    "Portable"
                } else {
                    "Local"
                },
                self.profile.path.display()
            ),
        );
        crate::desktops::follow(self.profiles);
        ShowWindow(self.profiles, SW_SHOWNORMAL);
        SetForegroundWindow(self.profiles);
    }
    fn backup_preferences(&self) -> Result<(), String> {
        profile::save(
            &profile::Active {
                path: self.directory.join("profile-backup.json"),
                id: Some("local-last-backup".into()),
            },
            &self.directory,
            &self.config,
        )
    }
    unsafe fn activate_preferences(&mut self, mut config: Config) {
        // The always-float list lives in config\always-float.json, not in the profile.
        config.always_float = std::mem::take(&mut self.config.always_float);
        if config.force_mastercam_ignore() {
            log("Imported profile had another Mastercam rule; set back to Ignore (always).");
            self.show_feedback("Mastercam rule set back to Ignore (always)");
        }
        if self.active {
            self.toggle();
        } else {
            self.restore();
        }
        if let Err(error) = self.opacity.restore_all() {
            message(&error);
        }
        let modifiers_changed = self.config.modifiers != config.modifiers
            || self.config.shortcut_overrides != config.shortcut_overrides
            || self.config.keymap != config.keymap
            || self.config.tap_super != config.tap_super;
        let cycle_changed = self.config.cycle_workspace_shortcut != config.cycle_workspace_shortcut;
        self.config = config;
        self.taskbar_hidden_preferred = self.config.taskbar_mode == "hidden";
        self.refresh_apps();
        let name = self.config.theme.clone();
        self.apply_theme(&name);
        bar::settings(&self.config.bar);
        bar::configure(
            self.config.show_top_bar,
            self.config.auto_hide_taskbar,
            &self.config.taskbar_mode,
            self.config.bar_opacity,
        );
        if cycle_changed {
            self.cycle_error = None;
            self.cycle_binding = None;
        }
        if modifiers_changed || cycle_changed {
            self.restart_keys();
            if !self.failures.is_empty() {
                log(&self.failures.join("\r\n"));
            }
        }
        overlay::set_font(&self.config.menu_font, self.config.menu_font_size);
        SendMessageW(
            GetDlgItem(self.appearance, 31),
            TBM_SETPOS,
            1,
            self.config.bar_opacity as isize,
        );
        SendMessageW(
            GetDlgItem(self.appearance, 32),
            TBM_SETPOS,
            1,
            self.config.inactive_opacity as isize,
        );
        if let Some(index) = theme::NAMES
            .iter()
            .position(|(name, _)| *name == self.config.theme)
        {
            SendMessageW(GetDlgItem(self.appearance, 30), CB_SETCURSEL, index, 0);
        }
        self.update_appearance();
        self.settings.refresh(&self.config, self.selected);
        self.show_profiles();
    }
    pub(super) unsafe fn profiles_command(&mut self, id: usize) -> Result<(), String> {
        match id {
            80 => {
                let path = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .parent()
                    .ok_or("Executable folder is unavailable.")?
                    .join(profile::FILE_NAME);
                if path.exists()
                    && path != self.profile.path
                    && MessageBoxW(
                        self.profiles,
                        wide("Replace the portable profile already in this folder?").as_ptr(),
                        wide(NAME).as_ptr(),
                        MB_YESNO | MB_ICONQUESTION,
                    ) != IDYES
                {
                    return Ok(());
                }
                let active = profile::Active {
                    path,
                    id: self.profile.id.clone().or_else(|| Some(profile::new_id())),
                };
                profile::save(&active, &self.directory, &self.config)?;
                self.profile = active;
                self.show_profiles();
            }
            81 => {
                if let Some(path) = profiles_ui::choose(self.profiles, true)? {
                    if path == self.profile.path
                        || fs::canonicalize(&path)
                            .ok()
                            .zip(fs::canonicalize(&self.profile.path).ok())
                            .is_some_and(|(a, b)| a == b)
                    {
                        return Err("Choose a separate export file; the active profile is saved through its normal settings controls.".into());
                    }
                    profile::write_json(
                        &path,
                        &profile::portable(&self.config, &profile::new_id()),
                    )?;
                }
            }
            82 => {
                if let Some(path) = profiles_ui::choose(self.profiles, false)? {
                    let mut imported = profile::Active {
                        path,
                        id: Some(String::new()),
                    };
                    let mut config = profile::load(&mut imported, &self.directory)?;
                    for rule in &mut config.rules {
                        if rule.monitor.is_none() {
                            rule.monitor = self
                                .config
                                .rules
                                .iter()
                                .find(|old| {
                                    old.executable.eq_ignore_ascii_case(&rule.executable)
                                        && old.title_contains == rule.title_contains
                                        && old.window_class == rule.window_class
                                })
                                .and_then(|old| old.monitor.clone());
                        }
                    }
                    self.backup_preferences()?;
                    profile::save(&self.profile, &self.directory, &config)?;
                    self.activate_preferences(config);
                }
            }
            83 => {
                let mut active = self.profile.clone();
                let config = profile::load(&mut active, &self.directory)?;
                self.backup_preferences()?;
                self.profile = active;
                self.activate_preferences(config);
            }
            84 => {
                if MessageBoxW(self.profiles,wide("Reset saved preferences and app rules to defaults? A backup will be kept in the local recovery folder.").as_ptr(),wide(NAME).as_ptr(),MB_YESNO|MB_ICONQUESTION)!=IDYES{return Ok(());}
                self.backup_preferences()?;
                let config = Config::default();
                profile::save(&self.profile, &self.directory, &config)?;
                self.activate_preferences(config);
            }
            85 => {
                if !shell(&self.directory, "") {
                    return Err("Could not open the local recovery folder.".into());
                }
            }
            86 => PostQuitMessage(0),
            87 => {
                let path = self.directory.join("recovery-report.json");
                if !path.exists() {
                    message("No recovery report has been recorded yet.");
                } else if !shell(&path, "") {
                    return Err("Could not open the recovery report.".into());
                }
            }
            _ => {}
        }
        Ok(())
    }
}
