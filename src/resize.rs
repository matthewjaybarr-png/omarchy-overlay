//! Follow mouse move and resize gestures, native or Super + drag, while keeping
//! neighboring tiles attached.
use super::*;
#[derive(Clone)]
pub(super) struct TiledGroup {
    pub keys: Vec<isize>,
    pub area: Rect,
    pub mode: String,
    pub gap: i32,
    pub ratio: f64,
    pub rects: Vec<Rect>,
}
pub(super) struct Drag {
    hwnd: HWND,
    monitor: isize,
    index: usize,
    base: TiledGroup,
    resizing: bool,
    ended: u8,
}
/// A fixed-size dialog such as a sign-in window is not tiled, but can still be moved.
unsafe fn movable(hwnd: HWND) -> bool {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    IsWindowVisible(hwnd) != 0 && pid != GetCurrentProcessId() && style & WS_CAPTION == WS_CAPTION
}

/// A Super + drag in progress: left moves the window, right resizes it from the corner
/// nearest the cursor.
pub(super) struct MouseDrag {
    hwnd: HWND,
    resize: bool,
    start: POINT,
    rect: RECT,
    right: bool,
    bottom: bool,
}

/// The window rect after the cursor moved by (dx, dy) during a drag.
fn dragged(d: &MouseDrag, dx: i32, dy: i32) -> RECT {
    let mut r = d.rect;
    if !d.resize {
        return RECT {
            left: r.left + dx,
            top: r.top + dy,
            right: r.right + dx,
            bottom: r.bottom + dy,
        };
    }
    let (min_w, min_h) = (200, 120);
    if d.right {
        r.right = (r.right + dx).max(r.left + min_w);
    } else {
        r.left = (r.left + dx).min(r.right - min_w);
    }
    if d.bottom {
        r.bottom = (r.bottom + dy).max(r.top + min_h);
    } else {
        r.top = (r.top + dy).min(r.bottom - min_h);
    }
    r
}

impl App {
    /// Handles keyhook::DRAG_MESSAGE.
    pub(super) unsafe fn mouse_drag_event(&mut self, kind: usize) {
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        match kind {
            keyhook::DRAG_MOVE | keyhook::DRAG_RESIZE => {
                self.mouse_drag = None;
                if !self.active {
                    return;
                }
                let hwnd = GetAncestor(WindowFromPoint(pt), GA_ROOT);
                let mut rect: RECT = zeroed();
                if hwnd.is_null()
                    || !(basic_eligible(hwnd) || kind == keyhook::DRAG_MOVE && movable(hwnd))
                    || IsZoomed(hwnd) != 0
                    || GetWindowRect(hwnd, &mut rect) == 0
                {
                    return;
                }
                SetForegroundWindow(hwnd);
                self.mouse_drag = Some(MouseDrag {
                    hwnd,
                    resize: kind == keyhook::DRAG_RESIZE,
                    start: pt,
                    rect,
                    right: pt.x * 2 > rect.left + rect.right,
                    bottom: pt.y * 2 > rect.top + rect.bottom,
                });
            }
            keyhook::DRAG_MOTION => {
                let Some(d) = &self.mouse_drag else {
                    return;
                };
                let r = dragged(d, pt.x - d.start.x, pt.y - d.start.y);
                let flags = if d.resize { 0 } else { SWP_NOSIZE };
                SetWindowPos(
                    d.hwnd,
                    null_mut(),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    flags | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
                );
            }
            _ => {
                let Some(d) = self.mouse_drag.take() else {
                    return;
                };
                let key = d.hwnd as isize;
                // Dropping a tile on another tile of its monitor swaps the two.
                let target = (!d.resize)
                    .then(|| self.tiled.values().find(|group| group.keys.contains(&key)))
                    .flatten()
                    .and_then(|group| {
                        group.keys.iter().zip(&group.rects).find_map(|(&other, r)| {
                            let inside =
                                pt.x >= r.x && pt.x < r.x + r.w && pt.y >= r.y && pt.y < r.y + r.h;
                            (other != key && inside).then_some(other)
                        })
                    });
                match target {
                    Some(other) => self.swap_tiles(key, other),
                    None => {
                        self.dirty = true;
                        self.tick();
                    }
                }
            }
        }
    }

    pub(super) unsafe fn resize_tick(&mut self) {
        if !self.active {
            self.drag = None;
            return;
        }
        let mut info: GUITHREADINFO = zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        let ours = self.mouse_drag.as_ref().map(|d| d.hwnd);
        let moving = ours.is_some()
            || GetGUIThreadInfo(0, &mut info) != 0 && info.flags & GUI_INMOVESIZE != 0;
        let moving_hwnd = ours.unwrap_or(info.hwndMoveSize);
        if self.drag.is_none() && moving {
            let hwnd = moving_hwnd;
            if !self
                .saved
                .get(&(hwnd as isize))
                .is_some_and(|saved| saved.matches(hwnd))
            {
                return;
            }
            if let Some((&monitor, group)) = self
                .tiled
                .iter()
                .find(|(_, group)| group.keys.contains(&(hwnd as isize)))
            {
                if group.mode == "scrolling" {
                    return;
                }
                let index = group
                    .keys
                    .iter()
                    .position(|key| *key == hwnd as isize)
                    .unwrap();
                self.drag = Some(Drag {
                    hwnd,
                    monitor,
                    index,
                    base: group.clone(),
                    resizing: false,
                    ended: 0,
                });
            }
        }
        let Some(mut drag) = self.drag.take() else {
            return;
        };
        if !self
            .saved
            .get(&(drag.hwnd as isize))
            .is_some_and(|saved| saved.matches(drag.hwnd))
            || IsIconic(drag.hwnd) != 0
        {
            self.dirty = true;
            return;
        }
        let active = moving && moving_hwnd == drag.hwnd;
        if !active {
            drag.ended += 1;
        }
        let mut frame: RECT = zeroed();
        if DwmGetWindowAttribute(
            drag.hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut frame as *mut _ as _,
            size_of::<RECT>() as u32,
        ) != 0
        {
            self.dirty = true;
            return;
        }
        let actual = Rect {
            x: frame.left,
            y: frame.top,
            w: frame.right - frame.left,
            h: frame.bottom - frame.top,
        };
        let original = drag.base.rects[drag.index];
        drag.resizing |= (actual.w - original.w).abs() > 4 || (actual.h - original.h).abs() > 4;
        if drag.resizing {
            let rects = crate::splits::resize(
                &drag.base.rects,
                drag.index,
                actual,
                drag.base.gap,
                (240, 140),
            );
            let changed = self
                .tiled
                .get(&drag.monitor)
                .is_none_or(|group| group.rects != rects);
            if changed || (!active && drag.ended >= 2) {
                for (index, (&key, &rect)) in drag.base.keys.iter().zip(&rects).enumerate() {
                    if index != drag.index || (!active && drag.ended >= 2) {
                        position(key as HWND, rect);
                    }
                }
            }
            if let Some(group) = self.tiled.get_mut(&drag.monitor) {
                group.rects = rects;
                if group.mode == "dwindle" {
                    if let Some(tree) = self.dwindle.get_mut(&drag.monitor) {
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
        }
        if active || drag.ended < 2 {
            self.drag = Some(drag);
        } else {
            self.dirty = true;
            self.tick();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drag(resize: bool, right: bool, bottom: bool) -> MouseDrag {
        MouseDrag {
            hwnd: null_mut(),
            resize,
            start: POINT { x: 0, y: 0 },
            rect: RECT {
                left: 100,
                top: 100,
                right: 500,
                bottom: 400,
            },
            right,
            bottom,
        }
    }
    fn tuple(r: RECT) -> (i32, i32, i32, i32) {
        (r.left, r.top, r.right, r.bottom)
    }

    #[test]
    fn moves_keep_size_and_resizes_move_the_grabbed_corner() {
        assert_eq!(
            tuple(dragged(&drag(false, true, true), 10, -20)),
            (110, 80, 510, 380)
        );
        assert_eq!(
            tuple(dragged(&drag(true, true, true), 50, 30)),
            (100, 100, 550, 430)
        );
        assert_eq!(
            tuple(dragged(&drag(true, false, false), -50, -30)),
            (50, 70, 500, 400)
        );
        // Shrinking stops at the minimum size.
        assert_eq!(
            tuple(dragged(&drag(true, true, false), -1000, 1000)),
            (100, 280, 300, 400)
        );
    }
}
