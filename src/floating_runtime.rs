//! The floating layout (Omarchy's floating workspace mode): a window that arrives on a
//! floating workspace is placed once at a comfortable centred size, then left where the
//! user puts it. Switching the workspace back to a tiling layout re-tiles those windows;
//! windows floated on their own account (Super + T, app rules) are never in the group.
use super::*;
impl App {
    pub(super) unsafe fn float_layout(&mut self, monitor: isize, group: &[(isize, isize, Rect)], area: Rect) {
        // Not a tiled group: native and Super + drags move these windows freely.
        self.tiled.remove(&monitor);
        self.dwindle.remove(&monitor);
        let mut placed = group
            .iter()
            .filter(|(key, _, _)| self.float_placed.contains(key))
            .count();
        for &(key, _, _) in group {
            if self.float_placed.contains(&key) {
                continue;
            }
            let hwnd = key as HWND;
            // The first placement is journaled like a tiled one, so exit, a crash and
            // RECOVER.cmd still put the window back where it was before the overlay.
            if !self.snapshot(hwnd) {
                continue;
            }
            self.float_placed.insert(key);
            // A maximized window stays maximized; the mode only floats tiled windows.
            if IsZoomed(hwnd) != 0 {
                continue;
            }
            self.arrange_window(hwnd, layout::floating(area, placed));
            placed += 1;
        }
    }
}
