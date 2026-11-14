use super::*;
#[derive(Clone)]
pub(super) struct Scratchpad {
    pub task: tasks::Task,
    pub hidden_by_us: bool,
}
/// Marks a popped-out window, so exit and crash recovery can find it and drop always-on-top.
const POP: &str = "OmarchyOverlay.Pop.v1";
const KEEP: u32 = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
unsafe extern "system" fn unpop(hwnd: HWND, _: LPARAM) -> i32 {
    if !RemovePropW(hwnd, wide(POP).as_ptr()).is_null() {
        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, KEEP);
    }
    1
}
/// A popped-out window is pinned: it stays on screen across workspaces.
pub(super) unsafe fn popped(hwnd: HWND) -> bool {
    !GetPropW(hwnd, wide(POP).as_ptr()).is_null()
}
pub(super) unsafe fn unpop_all() {
    EnumWindows(Some(unpop), 0);
}
impl App {
    /// Omarchy's Super+O: floats the window at 1300x900 in the middle of its monitor, on top of
    /// the tiles and on every workspace; again puts it back.
    pub(super) unsafe fn pop_window(&mut self, hwnd: HWND) {
        if !self.active || !self.movable(hwnd) {
            return;
        }
        let key = hwnd as isize;
        let tiled = self.order.contains(&key) && !self.floating.contains(&key);
        if !responsive(hwnd) {
            return;
        }
        // The mark is 1 for a window that was tiled before the pop and 2 for a floating one.
        let was = RemovePropW(hwnd, wide(POP).as_ptr()) as usize;
        if was != 0 {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, KEEP);
            if was == 1 && self.floating.contains(&key) {
                self.float_window(hwnd);
            }
            return;
        }
        // A window already always on top is someone else's: leaving it would lose that on unpop.
        if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0 {
            return;
        }
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut info) == 0 {
            return;
        }
        if tiled {
            self.float_window(hwnd);
        }
        let area = info.rcWork;
        let (width, height) = (1300.min(area.right - area.left), 900.min(area.bottom - area.top));
        SetPropW(hwnd, wide(POP).as_ptr(), if tiled { 1 } else { 2 } as HANDLE);
        if IsZoomed(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            (area.left + area.right - width) / 2,
            (area.top + area.bottom - height) / 2,
            width,
            height,
            SWP_NOACTIVATE,
        );
    }
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
