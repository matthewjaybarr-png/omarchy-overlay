use super::*;
impl App {
    pub(super) unsafe fn show_help(&self) {
        shortcuts_ui::fill(self, None);
        panel_scroll::fit(self.help);
        crate::desktops::follow(self.help);
        ShowWindow(self.help, SW_SHOWNORMAL);
        SetForegroundWindow(self.help);
    }
    pub(super) unsafe fn update_dashboard(&self) {
        if IsWindowVisible(self.main) == 0 {
            return;
        }
        let modifier = match self.config.modifiers.as_str() {
            "CtrlAlt" => "Ctrl + Alt",
            "Alt" => "Alt",
            _ => "Win + Alt",
        };
        set_text(
            GetDlgItem(self.main, 23),
            &format!(
                "{modifier} + 1–4: desktops · T: tiling · Space: app search · W: open windows"
            ),
        );
        set_text(
            GetDlgItem(self.main, 22),
            &format!(
                "Layout: {}   ·   Main pane: {}%   ·   Gap: {} px",
                self.layout_for_monitor(MonitorFromWindow(
                    if IsWindow(self.selected) != 0 {
                        self.selected
                    } else {
                        self.main
                    },
                    MONITOR_DEFAULTTONEAREST
                )),
                (self.config.main_ratio * 100.0).round() as i32,
                self.config.gap
            ),
        );
        if !self.failures.is_empty() {
            set_text(
                GetDlgItem(self.main, 23),
                "Some shortcuts are unavailable — open All keyboard shortcuts for their status.",
            );
        }
        let valid = basic_eligible(self.selected) && !elevated(self.selected);
        set_text(
            GetDlgItem(self.main, 21),
            &if valid {
                format!("Selected: {}", text_of(self.selected))
            } else {
                "Focus an app window, then open this menu to select it.".into()
            },
        );
        EnableWindow(
            GetDlgItem(self.main, 16),
            i32::from(self.active && self.order.contains(&(self.selected as isize))),
        );
        EnableWindow(GetDlgItem(self.main, 17), i32::from(valid && self.active));
        EnableWindow(GetDlgItem(self.main, 27), i32::from(self.active));
        let multiple = settings::displays().len() > 1;
        for id in [18, 19] {
            EnableWindow(GetDlgItem(self.main, id), i32::from(valid && multiple));
        }
        set_text(
            GetDlgItem(self.main, 17),
            if self.floating.contains(&(self.selected as isize)) {
                "Return window to tiling"
            } else {
                "Float selected window"
            },
        );
    }
    pub(super) unsafe fn movable(&self, hwnd: HWND) -> bool {
        basic_eligible(hwnd)
            && !elevated(hwnd)
            && !self
                .config
                .excluded_processes
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&process_name(hwnd)))
            && window_policy(&self.config, hwnd).mode != Mode::Ignore
    }
    pub(super) unsafe fn float_window(&mut self, hwnd: HWND) {
        if !self.active || !self.movable(hwnd) {
            return;
        }
        let key = hwnd as isize;
        if self.floating.remove(&key) {
            self.dirty = true;
        } else if self.order.contains(&key) {
            self.floating.insert(key);
            self.restore_one(key);
            self.dirty = true;
        } else {
            return;
        }
        self.tick();
        self.update_dashboard();
    }
    pub(super) unsafe fn promote(&mut self, hwnd: HWND) {
        if !self.active || !self.movable(hwnd) {
            return;
        }
        if let Some(index) = self.order.iter().position(|key| *key == hwnd as isize) {
            let key = self.order.remove(index);
            self.order.insert(0, key);
            self.dirty = true;
            self.tick();
            self.update_dashboard();
        }
    }
    pub(super) unsafe fn set_layout(&mut self, layout: &str) {
        if ![
            "main-stack",
            "columns",
            "rows",
            "grid",
            "dwindle",
            "scrolling",
        ]
        .contains(&layout)
        {
            return;
        }
        let monitor = MonitorFromWindow(
            if IsWindow(self.selected) != 0 {
                self.selected
            } else {
                self.main
            },
            MONITOR_DEFAULTTONEAREST,
        );
        if let Some((desktop, identity)) = self.layout_scope(monitor) {
            if !self
                .layout_preferences
                .effective(&desktop, &identity, "")
                .is_empty()
            {
                let mut choices = self.layout_preferences.clone();
                if let Err(error) = choices
                    .set(desktop, identity, Some(layout.into()))
                    .and_then(|_| self.save_layout_choices(choices))
                {
                    message(&error);
                }
                return;
            }
        }
        log(&format!("Layout selected by action: {layout}"));
        self.config.layout = layout.into();
        self.save_config();
        self.dirty = true;
        self.tick();
        self.update_dashboard();
        if IsWindowVisible(self.settings.hwnd) != 0 {
            self.settings.refresh(&self.config, self.selected);
        }
    }
    pub(super) unsafe fn cycle_layout(&mut self) {
        let mode = self.layout_for_monitor(MonitorFromWindow(
            if IsWindow(self.selected) != 0 {
                self.selected
            } else {
                GetForegroundWindow()
            },
            MONITOR_DEFAULTTONEAREST,
        ));
        let next = match mode.as_str() {
            "main-stack" => "columns",
            "columns" => "rows",
            "rows" => "grid",
            "grid" => "dwindle",
            "dwindle" => "scrolling",
            _ => "main-stack",
        };
        self.set_layout(next);
    }
    pub(super) unsafe fn rebalance(&mut self) {
        if !self.active {
            return;
        }
        self.drag = None;
        self.tiled.clear();
        self.dwindle.clear();
        self.dirty = true;
        self.tick();
        self.update_dashboard();
    }
    pub(super) unsafe fn change_split(&mut self, hwnd: HWND, flip: bool, delta: f64) {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) as isize;
        if !self.active || self.layout_for_monitor(monitor as HMONITOR) != "dwindle" {
            return;
        }
        if let (Some(tree), Some(group)) =
            (self.dwindle.get_mut(&monitor), self.tiled.get(&monitor))
        {
            if tree.change(hwnd as isize, flip, delta, group.area, group.gap) {
                self.dirty = true;
                self.tick();
            }
        }
    }
    pub(super) unsafe fn move_monitor(&mut self, hwnd: HWND, direction: i32) {
        if !self.movable(hwnd) || !responsive(hwnd) {
            return;
        }
        let mut displays = settings::displays();
        displays.sort_by_key(|display| (display.work.x, display.work.y));
        if displays.len() < 2 {
            return;
        }
        let current = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let Some(index) = displays
            .iter()
            .position(|display| display.monitor == current)
        else {
            return;
        };
        let next = (index as i32 + direction).rem_euclid(displays.len() as i32) as usize;
        if self.active && !self.snapshot(hwnd) {
            return;
        }
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return;
        }
        let target = layout::transfer(
            Rect {
                x: rect.left,
                y: rect.top,
                w: rect.right - rect.left,
                h: rect.bottom - rect.top,
            },
            displays[index].work,
            displays[next].work,
        );
        if IsZoomed(hwnd) != 0 {
            ShowWindowAsync(hwnd, SW_RESTORE);
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        // An explicit move overrides an unfinished preferred-monitor adoption.
        if let Some(device) = rule_for(&self.config, hwnd).and_then(|rule| rule.monitor.clone()) {
            self.routed.insert(
                hwnd as isize,
                Route {
                    pid,
                    device,
                    pending: 0,
                },
            );
        }
        self.moves.insert(
            hwnd as isize,
            Route {
                pid,
                device: displays[next].device.clone(),
                pending: 6,
            },
        );
        position(hwnd, target);
        SetForegroundWindow(hwnd);
        self.dirty = true;
    }
    pub(super) unsafe fn pending_move(
        &mut self,
        hwnd: HWND,
        displays: &[settings::Display],
    ) -> bool {
        let key = hwnd as isize;
        let Some(route) = self.moves.get_mut(&key) else {
            return false;
        };
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let confirmed = displays
            .iter()
            .find(|display| display.device == route.device)
            .is_none_or(|display| {
                MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == display.monitor
            });
        if pid != route.pid || confirmed || route.pending == 0 {
            self.moves.remove(&key);
            return false;
        }
        route.pending -= 1;
        true
    }
}

impl App {
    pub(super) unsafe fn set_taskbar_policy(&mut self, mode: &str) {
        match bar::set_taskbar_mode(mode) {
            Ok(()) => {
                self.config.taskbar_mode = mode.into();
                self.config.auto_hide_taskbar = mode != "visible";
                if mode == "hidden" {
                    self.taskbar_hidden_preferred = true
                } else if mode == "auto-hide" {
                    self.taskbar_hidden_preferred = false
                }
                self.save_config();
            }
            Err(e) => message(&e),
        }
    }
    pub(super) unsafe fn toggle_windows_taskbar(&mut self) {
        let mode = if self.config.taskbar_mode == "hidden" {
            "visible"
        } else if self.taskbar_hidden_preferred {
            "hidden"
        } else if self.config.auto_hide_taskbar {
            "visible"
        } else {
            "auto-hide"
        };
        self.set_taskbar_policy(mode);
    }
}
