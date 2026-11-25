//! Feeds the dock: pins from the app picker's favorites, running windows from the same
//! list the bar's Tasks module uses, and what a click on an icon does.
use super::*;
use std::rc::Rc;

/// What a dock icon stands for, kept for its clicks.
pub(super) struct DockAction {
    /// The app picker entry it launches, by key.
    entry: Option<String>,
    tasks: Vec<tasks::Task>,
}
impl App {
    /// The lowercase ids that claim a window for a picker entry: an app ID and/or a
    /// program file name. Cached: reading a shortcut goes through the Shell.
    unsafe fn entry_ids(&mut self, index: usize) -> Vec<String> {
        let entry = &self.entries[index];
        let key = entry.key();
        if let Some(ids) = self.dock_entry_ids.get(&key) {
            return ids.clone();
        }
        let file = |text: &str| {
            Path::new(text.trim_matches('"'))
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .filter(|n| n.ends_with(".exe"))
        };
        let mut ids = vec![];
        if entry.action.is_none() {
            let arguments = entry.arguments.trim_matches('"');
            if let Some(target) = arguments
                .strip_prefix("shell:AppsFolder\\")
                .or_else(|| arguments.strip_prefix("shell:appsfolder\\"))
            {
                ids.push(target.to_lowercase());
                ids.extend(file(target));
            } else if entry
                .path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
            {
                ids.extend(native_status::shortcut_app_id(&entry.path).map(|id| id.to_lowercase()));
                ids.extend(native_status::shortcut_target(&entry.path).and_then(|t| file(&t)));
            } else {
                ids.extend(file(&entry.path.to_string_lossy()));
            }
        }
        self.dock_entry_ids.insert(key, ids.clone());
        ids
    }
    /// A window's ids, most specific first: its app ID, then its program file name.
    /// A browser web app keeps only its own ID, so it never joins the browser's icon.
    unsafe fn window_ids(&mut self, task: &tasks::Task) -> Vec<String> {
        let key = (task.hwnd as isize, task.pid);
        if let Some(ids) = self.dock_window_ids.get(&key) {
            return ids.clone();
        }
        let app = native_status::window_app(task.hwnd)
            .0
            .map(|id| id.to_lowercase());
        let mut ids: Vec<String> = app.iter().cloned().collect();
        if !app.as_deref().is_some_and(tasks::web_app_id) {
            ids.extend(
                process_path(task.hwnd)
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase())),
            );
        }
        self.dock_window_ids.insert(key, ids.clone());
        ids
    }
    /// A running app's picture at `px`: drawn by the Shell from its program file, or its
    /// window icon when the Shell has none.
    unsafe fn task_picture(&mut self, task: &tasks::Task, px: i32) -> Rc<picker_ui::Icon> {
        let path = process_path(task.hwnd);
        let key = format!(
            "dock\n{}@{px}",
            path.as_ref().map_or_else(|| format!("{:?}", task.hwnd), |p| p.to_string_lossy().to_lowercase())
        );
        if let Some(icon) = self.picker_icons.get(&key) {
            return icon.clone();
        }
        let big = path
            .map(|p| native_status::shell_picture(&p.to_string_lossy(), px))
            .unwrap_or(null_mut());
        let icon = Rc::new(picker_ui::Icon(if big.is_null() {
            CopyIcon(task.identity.icon)
        } else {
            big
        }));
        // A web app's picture is its window's own icon; don't share it by program.
        if !task.identity.source.starts_with("window") {
            self.picker_icons.insert(key, icon.clone());
        }
        icon
    }
    pub(super) unsafe fn dock_poll(&mut self) {
        let mode = self.config.dock.clone();
        let foreground = GetForegroundWindow();
        let mut monitors = vec![];
        if mode != "off" {
            for display in settings::displays() {
                let mut info: MONITORINFO = zeroed();
                info.cbSize = size_of::<MONITORINFO>() as u32;
                if GetMonitorInfoW(display.monitor, &mut info) == 0 {
                    continue;
                }
                let layout = if self.active {
                    self.layout_for_monitor(display.monitor)
                } else {
                    String::new()
                };
                let mut r: RECT = zeroed();
                let full_screen = !foreground.is_null()
                    && MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST) == display.monitor
                    && GetWindowRect(foreground, &mut r) != 0
                    && r.left <= info.rcMonitor.left
                    && r.top <= info.rcMonitor.top
                    && r.right >= info.rcMonitor.right
                    && r.bottom >= info.rcMonitor.bottom
                    && !["Progman", "WorkerW"].contains(&window_class(foreground).as_str());
                if crate::dock::shows(&mode, &layout, full_screen) {
                    monitors.push((display.monitor, info.rcWork, info.rcMonitor));
                }
            }
        }
        if monitors.is_empty() {
            self.dock_actions.clear();
            dock_ui::update(self.main, vec![], vec![], false);
            return;
        }
        let px = dock_ui::icon_px(monitors[0].0);
        let pinned: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.entries[i].action.is_none() && self.picker_history.favorite(&self.entries[i].key()))
            .collect();
        let pins: Vec<crate::dock::Pin> = pinned
            .iter()
            .map(|&i| crate::dock::Pin {
                key: self.entries[i].key(),
                ids: self.entry_ids(i),
            })
            .collect();
        let tasks = tasks::collect();
        self.dock_window_ids
            .retain(|&(hwnd, pid), _| tasks.iter().any(|t| t.hwnd as isize == hwnd && t.pid == pid));
        let windows: Vec<crate::dock::Window> = tasks
            .iter()
            .map(|task| crate::dock::Window {
                ids: self.window_ids(task),
                here: !self.parked.contains_key(&(task.hwnd as isize)),
                minimized: task.minimized,
                focused: task.hwnd == foreground,
            })
            .collect();
        let items = crate::dock::items(&pins, &windows);
        let mut views = vec![];
        self.dock_actions.clear();
        for item in items {
            let owned: Vec<tasks::Task> = item.windows.iter().map(|&i| tasks[i].clone()).collect();
            let (label, icon) = match item.pin {
                Some(pin) => (self.entries[pinned[pin]].label.clone(), self.entry_picture(pinned[pin], px)),
                None => (owned[0].identity.label.clone(), self.task_picture(&owned[0], px)),
            };
            views.push(dock_ui::View {
                label,
                icon,
                running: !owned.is_empty(),
                focused: item.windows.iter().any(|&i| windows[i].focused),
            });
            self.dock_actions.push(DockAction {
                entry: item.pin.map(|pin| self.entries[pinned[pin]].key()),
                tasks: owned,
            });
        }
        dock_ui::update(self.main, views, monitors, mode == "always");
    }
    unsafe fn dock_launch(&mut self, key: &str) -> bool {
        let Some(entry) = self.entries.iter().find(|e| e.key() == key).cloned() else {
            return false;
        };
        if !entry.available() {
            message(&format!(
                "This custom entry is unavailable on this PC. Check its path and working folder in Manage apps.\r\n{}",
                entry.path.display()
            ));
            return false;
        }
        if shell_in(&entry.path, &entry.arguments, entry.working_directory.as_deref()) {
            self.picker_history.launched(key);
            self.save_picker();
            true
        } else {
            message(&format!("Could not launch {}. Its shortcut may no longer be valid.", entry.label));
            false
        }
    }
    /// The picker entry that launches a running app that is not pinned, if any.
    unsafe fn dock_entry_for(&mut self, task: &tasks::Task) -> Option<String> {
        let ids = self.window_ids(task);
        for i in 0..self.entries.len() {
            if self.entries[i].action.is_some() {
                continue;
            }
            let theirs = self.entry_ids(i);
            if ids.iter().any(|id| theirs.contains(id)) {
                return Some(self.entries[i].key());
            }
        }
        None
    }
    /// A click on dock icon `index`: 0 left (bring the app forward or launch it),
    /// 1 middle (a new window), 2 right (a menu).
    pub(super) unsafe fn dock_click(&mut self, kind: usize, index: usize) {
        let Some(action) = self.dock_actions.get(index) else {
            return;
        };
        let entry = action.entry.clone();
        let tasks: Vec<tasks::Task> = action.tasks.iter().filter(|t| t.valid() || IsIconic(t.hwnd) != 0).cloned().collect();
        match kind {
            0 => {
                let foreground = GetForegroundWindow();
                let windows: Vec<crate::dock::Window> = tasks
                    .iter()
                    .map(|t| crate::dock::Window {
                        ids: vec![],
                        here: !self.parked.contains_key(&(t.hwnd as isize)),
                        minimized: IsIconic(t.hwnd) != 0,
                        focused: t.hwnd == foreground,
                    })
                    .collect();
                let item = crate::dock::Item {
                    key: String::new(),
                    pin: None,
                    windows: (0..tasks.len()).collect(),
                };
                match crate::dock::pick(&item, &windows) {
                    Some(i) => {
                        tasks::activate(&tasks[i], false);
                    }
                    None => {
                        if let Some(key) = entry {
                            self.dock_launch(&key);
                        }
                    }
                }
            }
            1 => self.dock_new_window(entry, &tasks),
            _ => self.dock_menu(entry, tasks),
        }
        self.dock_poll();
    }
    unsafe fn dock_new_window(&mut self, entry: Option<String>, tasks: &[tasks::Task]) {
        let entry = entry.or_else(|| tasks.first().and_then(|t| self.dock_entry_for(t)));
        if let Some(key) = entry {
            self.dock_launch(&key);
        } else if let Some(path) = tasks.first().and_then(|t| process_path(t.hwnd)) {
            shell_in(&path, "", None);
        }
    }
    unsafe fn dock_menu(&mut self, entry: Option<String>, tasks: Vec<tasks::Task>) {
        let pinned = entry.is_some();
        let key = entry.or_else(|| tasks.first().and_then(|t| self.dock_entry_for(t)));
        let menu = CreatePopupMenu();
        let launchable = key.is_some() || tasks.first().and_then(|t| process_path(t.hwnd)).is_some();
        let item = |id: usize, label: &str, enabled: bool| {
            AppendMenuW(
                menu,
                MF_STRING | if enabled { 0 } else { MF_GRAYED },
                id,
                wide(label).as_ptr(),
            );
        };
        item(1, "New window", launchable);
        item(
            2,
            if pinned { "Remove from dock" } else { "Keep in dock" },
            key.is_some(),
        );
        if !tasks.is_empty() {
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            item(3, if tasks.len() == 1 { "Close window" } else { "Close all windows" }, true);
        }
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(self.main);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_BOTTOMALIGN | TPM_CENTERALIGN,
            point.x,
            point.y,
            0,
            self.main,
            null(),
        );
        DestroyMenu(menu);
        match chosen {
            1 => self.dock_new_window(key, &tasks),
            2 => {
                if let Some(key) = key {
                    self.picker_history.toggle(&key);
                    self.save_picker();
                }
            }
            3 => {
                for task in &tasks {
                    if task.valid() || IsIconic(task.hwnd) != 0 {
                        PostMessageW(task.hwnd, WM_CLOSE, 0, 0);
                    }
                }
            }
            _ => {}
        }
    }
}
