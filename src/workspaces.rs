//! Omarchy Overlay's own workspaces. Windows has no documented way to move another app's window
//! between its virtual desktops, so the windows of a workspace that is not shown are parked off
//! the left edge of the screen. Each one is in the placement journal before it moves, so exit and
//! crash recovery bring every window back.
use super::*;
pub(super) struct Parked {
    workspace: usize,
    /// Where the window was when it was parked.
    rect: RECT,
    /// The journal entry was made for parking, not by tiling, and goes when the window returns.
    fresh: bool,
}
const MOVE: u32 = SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE;
impl App {
    /// Parks one window for `workspace`. False when the window is not one the overlay may move.
    unsafe fn park(&mut self, hwnd: HWND, workspace: usize) -> bool {
        let key = hwnd as isize;
        // A window the scrolling layout minimized stays minimized and returns with its workspace.
        let minimized = scrolling_runtime::current_hidden(hwnd);
        if self.parked.contains_key(&key)
            || window_actions::popped(hwnd)
            || !(minimized || self.movable(hwnd))
        {
            return false;
        }
        let fresh = !self.saved.get(&key).is_some_and(|saved| saved.matches(hwnd));
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 || !self.snapshot(hwnd) {
            return false;
        }
        self.parked.insert(key, Parked { workspace, rect, fresh });
        if !minimized {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN) - (rect.right - rect.left) - 64;
            SetWindowPos(hwnd, null_mut(), x, rect.top, 0, 0, MOVE | SWP_ASYNCWINDOWPOS);
        }
        true
    }
    unsafe fn unpark(&mut self, key: isize) {
        let Some(parked) = self.parked.remove(&key) else {
            return;
        };
        let hwnd = key as HWND;
        if IsWindow(hwnd) == 0 {
            return;
        }
        if IsIconic(hwnd) == 0 {
            // Synchronous where possible: the tiling pass that follows reads the window's monitor.
            let wait = if responsive(hwnd) { 0 } else { SWP_ASYNCWINDOWPOS };
            SetWindowPos(hwnd, null_mut(), parked.rect.left, parked.rect.top, 0, 0, MOVE | wait);
        }
        if parked.fresh && self.saved.remove(&key).is_some() {
            RemovePropW(hwnd, wide(PLACEMENT_OWNER).as_ptr());
            if let Err(error) = self.save_positions() {
                log(&error);
            }
        }
    }
    fn report_occupied(&mut self) {
        self.parked.retain(|&key, _| unsafe { IsWindow(key as HWND) } != 0);
        let occupied = self.parked.values().fold(0u16, |mask, p| mask | 1 << p.workspace);
        crate::desktops::set_occupied(occupied);
    }
    unsafe fn focus_or_desktop(&mut self, target: Option<isize>) {
        // Seen first, so a refused focus change cannot read as the user picking a parked window.
        let front = GetForegroundWindow();
        self.seen_foreground = front as isize;
        let target = target.map_or(GetShellWindow(), |key| key as HWND);
        if SetForegroundWindow(target) != 0 && GetForegroundWindow() == target {
            return;
        }
        // Outside a key press Windows refuses; joining the front window's input is the
        // documented way to hand focus over (as overlay::bring_to_front does).
        let theirs = GetWindowThreadProcessId(front, null_mut());
        let ours = GetCurrentThreadId();
        if theirs != 0 && theirs != ours && AttachThreadInput(ours, theirs, 1) != 0 {
            SetForegroundWindow(target);
            AttachThreadInput(ours, theirs, 0);
        }
    }
    /// Called at the start of every tiling pass: follows a parked window the user activated
    /// (Alt+Tab, the taskbar) to its workspace, then parks and returns windows after a switch.
    pub(super) unsafe fn sync_workspace(&mut self) {
        let foreground = GetForegroundWindow() as isize;
        if foreground != self.seen_foreground {
            let before = std::mem::replace(&mut self.seen_foreground, foreground) as HWND;
            if let Some(workspace) = self.parked.get(&foreground).map(|p| p.workspace) {
                // Windows hands focus to the next window when one closes or minimizes. Only a
                // window the user chose, with the one before still there, changes workspace.
                if IsWindow(before) != 0 && IsWindowVisible(before) != 0 && IsIconic(before) == 0 {
                    crate::desktops::select(workspace);
                    self.workspace_focus.insert(workspace, foreground);
                } else {
                    let next = self.order.iter().copied().find(|&key| IsWindow(key as HWND) != 0);
                    self.focus_or_desktop(next);
                }
            }
        }
        let next = crate::desktops::current();
        if next == self.workspace {
            return;
        }
        let old = std::mem::replace(&mut self.workspace, next);
        if !self.parked.contains_key(&foreground) {
            self.workspace_focus.insert(old, foreground);
        }
        let mut found: Vec<isize> = vec![];
        unsafe extern "system" fn enumerate(hwnd: HWND, parameter: LPARAM) -> i32 {
            (*(parameter as *mut Vec<isize>)).push(hwnd as isize);
            1
        }
        EnumWindows(Some(enumerate), &mut found as *mut _ as isize);
        for &key in &found {
            self.park(key as HWND, old);
        }
        // EnumWindows runs top to bottom, so the first window back is the one that was on top.
        let back: Vec<isize> = found
            .iter()
            .copied()
            .filter(|key| self.parked.get(key).is_some_and(|p| p.workspace == next))
            .collect();
        for &key in &back {
            self.unpark(key);
        }
        let target = self
            .workspace_focus
            .get(&next)
            .copied()
            .filter(|key| back.contains(key))
            .or(back.iter().copied().find(|&key| IsIconic(key as HWND) == 0));
        self.focus_or_desktop(target);
        self.report_occupied();
    }
    /// Omarchy's Super+Shift+number: sends the focused window to a workspace and, unless
    /// `silent`, goes there with it.
    pub(super) unsafe fn move_to_workspace(&mut self, workspace: usize, silent: bool) {
        let hwnd = GetForegroundWindow();
        if !self.active || workspace == self.workspace || !self.park(hwnd, workspace) {
            return;
        }
        let key = hwnd as isize;
        self.order.retain(|other| *other != key);
        self.report_occupied();
        if silent {
            self.focus_or_desktop(self.order.first().copied());
        } else {
            self.workspace_focus.insert(workspace, key);
            crate::desktops::select(workspace);
        }
        self.dirty = true;
        self.tick();
        bar::poll();
    }
    /// Tiling stopped: the journal has just put every window back.
    pub(super) fn forget_workspaces(&mut self) {
        self.parked.clear();
        self.workspace_focus.clear();
        crate::desktops::set_occupied(0);
    }
}
