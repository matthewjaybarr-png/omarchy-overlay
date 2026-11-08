use super::*;
#[derive(Clone)]
pub(super) struct Scratchpad {
    pub task: tasks::Task,
    pub hidden_by_us: bool,
}
impl App {
    pub(super) unsafe fn move_direction(&mut self, hwnd: HWND, dx: i32, dy: i32) {
        let mut displays = settings::displays();
        displays.sort_by_key(|d| (d.work.x, d.work.y));
        let current = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let Some(index) = displays.iter().position(|d| d.monitor == current) else {
            return;
        };
        let rects = displays.iter().map(|d| d.work).collect::<Vec<_>>();
        if let Some(next) = layout::directional(&rects, index, dx, dy) {
            self.move_monitor(hwnd, next as i32 - index as i32);
        }
    }
    pub(super) unsafe fn close_window(&self, hwnd: HWND) {
        if self.movable(hwnd) {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
    }
    pub(super) unsafe fn maximize_window(&mut self, hwnd: HWND) {
        if !self.movable(hwnd) || !self.snapshot(hwnd) {
            return;
        }
        self.floating.insert(hwnd as isize);
        self.dirty = true;
        ShowWindowAsync(
            hwnd,
            if IsZoomed(hwnd) != 0 {
                SW_RESTORE
            } else {
                SW_MAXIMIZE
            },
        );
    }
    pub(super) unsafe fn resize_keyboard(&mut self, hwnd: HWND, horizontal: bool, delta: i32) {
        if !self.active || !self.movable(hwnd) {
            return;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) as isize;
        let Some(group) = self.tiled.get_mut(&monitor) else {
            return;
        };
        let Some(index) = group.keys.iter().position(|&key| key == hwnd as isize) else {
            return;
        };
        let rects = crate::splits::keyboard(
            &group.rects,
            index,
            horizontal,
            delta,
            group.gap,
            (240, 140),
        );
        for (&key, &rect) in group.keys.iter().zip(&rects) {
            position(key as HWND, rect);
        }
        group.rects = rects;
        if group.mode == "dwindle" {
            if let Some(tree) = self.dwindle.get_mut(&monitor) {
                tree.adopt(
                    &group
                        .keys
                        .iter()
                        .copied()
                        .zip(group.rects.iter().copied())
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    pub(super) unsafe fn opacity_preset(&mut self, hwnd: HWND, percent: u8) {
        let name = process_name(hwnd).to_ascii_lowercase();
        if !basic_eligible(hwnd)
            || elevated(hwnd)
            || !can_change_opacity(&self.config, hwnd)
            || name.starts_with("mastercam")
            || [
                "windowsterminal.exe",
                "windowsterminalpreview.exe",
                "openconsole.exe",
            ]
            .contains(&name.as_str())
        {
            return;
        }
        let desired = if self.opacity.percent(hwnd) == percent {
            100
        } else {
            percent
        };
        if let Err(error) = self.opacity.manual(hwnd, desired) {
            message(&error);
        }
        self.selected = hwnd;
        self.update_appearance();
    }
    pub(super) unsafe fn assign_scratchpad(&mut self, hwnd: HWND) {
        if !self.movable(hwnd) {
            return;
        }
        let Some(desktop) = workspace_ui::current_desktop() else {
            return;
        };
        let Some(task) = tasks::collect().into_iter().find(|task| task.hwnd == hwnd) else {
            return;
        };
        if let Some(old) = self.scratchpads.remove(&desktop) {
            if old.hidden_by_us && old.task.valid() {
                ShowWindowAsync(old.task.hwnd, SW_RESTORE);
            }
        }
        self.floating.remove(&(hwnd as isize));
        self.restore_one(hwnd as isize);
        self.scratchpads.insert(
            desktop,
            Scratchpad {
                task,
                hidden_by_us: false,
            },
        );
        self.dirty = true;
        self.tick();
    }
    pub(super) unsafe fn toggle_scratchpad(&mut self) {
        let Some(desktop) = workspace_ui::current_desktop() else {
            return;
        };
        let Some(pad) = self.scratchpads.get_mut(&desktop) else {
            message("No scratchpad is assigned on this Windows desktop. Focus a utility window and use Assign scratchpad.");
            return;
        };
        if !pad.task.valid() {
            self.scratchpads.remove(&desktop);
            message("That scratchpad window is no longer available. Assign another window on this desktop.");
            return;
        }
        if IsIconic(pad.task.hwnd) != 0 || GetForegroundWindow() != pad.task.hwnd {
            pad.hidden_by_us = false;
            tasks::activate(&pad.task, false);
        } else {
            pad.hidden_by_us = true;
            ShowWindowAsync(pad.task.hwnd, SW_MINIMIZE);
        }
    }
    pub(super) unsafe fn restore_scratchpads(&mut self) {
        for pad in self.scratchpads.values() {
            let mut pid = 0;
            GetWindowThreadProcessId(pad.task.hwnd, &mut pid);
            if pad.hidden_by_us && IsWindow(pad.task.hwnd) != 0 && pid == pad.task.pid {
                ShowWindowAsync(pad.task.hwnd, SW_RESTORE);
            }
        }
        self.scratchpads.clear();
    }
}
