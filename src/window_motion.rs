use super::*;
pub(super) struct Motion {
    tween: crate::motion::Tween,
    started: std::time::Instant,
}
unsafe fn allowed() -> bool {
    let mut enabled = 0i32;
    SystemParametersInfoW(
        SPI_GETCLIENTAREAANIMATION,
        0,
        &mut enabled as *mut _ as _,
        0,
    ) != 0
        && enabled != 0
}
impl App {
    pub(super) unsafe fn arrange_window(&mut self, h: HWND, target: Rect) {
        if !self.config.animations
            || !allowed()
            || self.drag.is_some()
            || GetAsyncKeyState(VK_LBUTTON as i32) < 0
        {
            self.motions.remove(&(h as isize));
            position(h, target);
            return;
        }
        let mut r: RECT = zeroed();
        if DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut r as *mut _ as _,
            size_of::<RECT>() as u32,
        ) < 0
        {
            GetWindowRect(h, &mut r);
        }
        let from = Rect {
            x: r.left,
            y: r.top,
            w: r.right - r.left,
            h: r.bottom - r.top,
        };
        if from == target {
            return;
        }
        self.motions.insert(
            h as isize,
            Motion {
                tween: crate::motion::Tween {
                    from,
                    to: target,
                    milliseconds: self.config.animation_duration_ms,
                },
                started: std::time::Instant::now(),
            },
        );
        SetTimer(self.main, 4, 16, None);
    }
    pub(super) unsafe fn finish_motion(&mut self, finish: bool) {
        for (key, motion) in self.motions.drain() {
            if finish && self.saved.get(&key).is_some_and(|s| s.matches(key as HWND)) {
                position(key as HWND, motion.tween.to);
            }
        }
        KillTimer(self.main, 4);
    }
    pub(super) unsafe fn motion_tick(&mut self) {
        let mut info: GUITHREADINFO = zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if !self.config.animations
            || !allowed()
            || GetGUIThreadInfo(0, &mut info) != 0 && info.flags & GUI_INMOVESIZE != 0
        {
            self.finish_motion(true);
            return;
        }
        if !self.active {
            self.finish_motion(false);
            return;
        }
        let saved = &self.saved;
        self.motions.retain(|&key, m| {
            let h = key as HWND;
            if !saved.get(&key).is_some_and(|s| s.matches(h)) || IsIconic(h) != 0 {
                return false;
            }
            let elapsed = m.started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
            position(h, m.tween.at(elapsed));
            elapsed < m.tween.milliseconds
        });
        if self.motions.is_empty() {
            KillTimer(self.main, 4);
        }
    }
}
