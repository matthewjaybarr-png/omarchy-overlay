//! Follow native mouse resize gestures while keeping neighboring tiles attached.
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
impl App {
    pub(super) unsafe fn resize_tick(&mut self) {
        if !self.active || crate::desktops::switching() {
            self.drag = None;
            return;
        }
        let mut info: GUITHREADINFO = zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        let moving = GetGUIThreadInfo(0, &mut info) != 0 && info.flags & GUI_INMOVESIZE != 0;
        if self.drag.is_none() && moving {
            let hwnd = info.hwndMoveSize;
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
        let active = moving && info.hwndMoveSize == drag.hwnd;
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
