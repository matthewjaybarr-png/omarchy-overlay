//! Windows adaptation of a horizontal scrolling viewport. Off-view windows use
//! ordinary minimization, identified by a property and the existing local restore journal.
use super::*;
pub(super) const HIDDEN: &str = "OmarchyOverlay.ScrollHidden.v1";
pub(super) unsafe fn owned(hwnd: HWND) -> bool {
    let owner = GetPropW(hwnd, wide(PLACEMENT_OWNER).as_ptr());
    !owner.is_null() && owner == GetPropW(hwnd, wide(HIDDEN).as_ptr())
}
pub(super) unsafe fn current_hidden(hwnd: HWND) -> bool {
    if !owned(hwnd)
        || GetPropW(hwnd, wide(HIDDEN).as_ptr()) as usize != GetCurrentProcessId() as usize
    {
        return false;
    }
    let mut cloaked = 0u32;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut _ as _,
        size_of::<u32>() as u32,
    );
    cloaked == 0
}
impl App {
    pub(super) unsafe fn release_scrolling_for_other_layouts(&mut self) {
        for (&key, saved) in &self.saved {
            let hwnd = key as HWND;
            if saved.matches(hwnd)
                && current_hidden(hwnd)
                && self.layout_for_monitor(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST))
                    != "scrolling"
            {
                if IsIconic(hwnd) != 0 {
                    ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
                    self.dirty = true;
                } else {
                    RemovePropW(hwnd, wide(HIDDEN).as_ptr());
                }
            }
        }
    }
    pub(super) unsafe fn tile_scrolling(
        &mut self,
        monitor: isize,
        group: &[(isize, isize, Rect)],
        area: Rect,
    ) {
        let keys = group.iter().map(|(key, _, _)| *key).collect::<Vec<_>>();
        if keys.is_empty() {
            return;
        }
        let foreground = GetForegroundWindow() as isize;
        let state = self
            .scrolling
            .entry((self.layout_desktop.clone(), monitor))
            .or_default();
        if keys.contains(&foreground) {
            state.active = foreground;
        } else if !keys.contains(&state.active) {
            state.active = keys[0];
        }
        let focus = keys
            .iter()
            .position(|key| *key == state.active)
            .unwrap_or(0);
        let view = crate::scrolling::viewport(
            area,
            keys.len(),
            focus,
            state.start,
            self.config.scrolling_columns,
            self.config.gap,
        );
        state.start = view.start;
        let visible = view
            .visible
            .iter()
            .map(|(i, _)| keys[*i])
            .collect::<Vec<_>>();
        for (i, key) in keys.iter().enumerate() {
            let hwnd = *key as HWND;
            if !self.snapshot(hwnd) {
                continue;
            }
            if let Some((_, target)) = view.visible.iter().find(|(index, _)| *index == i) {
                if IsIconic(hwnd) != 0 || IsZoomed(hwnd) != 0 {
                    ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
                    self.dirty = true;
                }
                if owned(hwnd) && IsIconic(hwnd) == 0 {
                    RemovePropW(hwnd, wide(HIDDEN).as_ptr());
                }
                // Immediate monitor-local placements avoid animating through another display.
                position(hwnd, *target);
            } else {
                let popup = GetLastActivePopup(hwnd);
                if !popup.is_null() && popup != hwnd && IsWindowVisible(popup) != 0 {
                    self.dirty = true;
                    continue;
                }
                let marker = self.saved.get(key).map_or(0, |saved| saved.marker);
                if !owned(hwnd) && SetPropW(hwnd, wide(HIDDEN).as_ptr(), marker as HANDLE) == 0 {
                    log("Could not mark an off-view scrolling window; left it visible.");
                    continue;
                }
                if IsIconic(hwnd) == 0 {
                    ShowWindowAsync(hwnd, SW_SHOWMINNOACTIVE);
                }
            }
        }
        self.tiled.insert(
            monitor,
            resize::TiledGroup {
                keys: visible,
                area,
                mode: "scrolling".into(),
                gap: self.config.gap,
                ratio: self.config.main_ratio,
                rects: view.visible.iter().map(|(_, rect)| *rect).collect(),
            },
        );
    }
    pub(super) unsafe fn scroll_step(
        &mut self,
        monitor: HMONITOR,
        direction: i32,
        swap: bool,
    ) -> bool {
        if !self.active || self.layout_for_monitor(monitor) != "scrolling" {
            return false;
        }
        let keys = self
            .order
            .iter()
            .copied()
            .filter(|key| {
                let hwnd = *key as HWND;
                IsWindow(hwnd) != 0
                    && MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) == monitor
                    && (IsIconic(hwnd) == 0 || current_hidden(hwnd))
            })
            .collect::<Vec<_>>();
        if keys.is_empty() {
            return true;
        }
        let foreground = GetForegroundWindow() as isize;
        let state = self
            .scrolling
            .entry((self.layout_desktop.clone(), monitor as isize))
            .or_default();
        let current = if keys.contains(&foreground) {
            foreground
        } else {
            state.active
        };
        let index = keys.iter().position(|key| *key == current).unwrap_or(0);
        let next = (index as i32 + direction.signum()).clamp(0, keys.len() as i32 - 1) as usize;
        let target = keys[next];
        if swap {
            if let (Some(a), Some(b)) = (
                self.order.iter().position(|key| *key == current),
                self.order.iter().position(|key| *key == target),
            ) {
                self.order.swap(a, b);
            }
        } else {
            state.active = target;
            let hwnd = target as HWND;
            if IsIconic(hwnd) != 0 {
                ShowWindowAsync(hwnd, SW_RESTORE);
            }
            SetForegroundWindow(hwnd);
        }
        self.dirty = true;
        self.tick();
        if !swap {
            SetForegroundWindow(target as HWND);
        }
        true
    }
}
