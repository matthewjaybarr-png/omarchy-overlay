//! Bounded interpolation for optional tile transitions.
use crate::layout::Rect;
#[derive(Clone, Copy)]
pub struct Tween {
    pub from: Rect,
    pub to: Rect,
    pub milliseconds: u32,
}
impl Tween {
    pub fn at(&self, elapsed: u32) -> Rect {
        if elapsed >= self.milliseconds {
            return self.to;
        }
        let t = f64::from(elapsed) / f64::from(self.milliseconds.max(1));
        let ease = 1.0 - (1.0 - t).powi(3);
        let lerp =
            |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * ease).round() as i32;
        Rect {
            x: lerp(self.from.x, self.to.x),
            y: lerp(self.from.y, self.to.y),
            w: lerp(self.from.w, self.to.w).max(1),
            h: lerp(self.from.h, self.to.h).max(1),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transitions_are_bounded_monotonic_and_finish_exactly() {
        let t = Tween {
            from: Rect {
                x: -1920,
                y: -120,
                w: 600,
                h: 400,
            },
            to: Rect {
                x: 10,
                y: 50,
                w: 1200,
                h: 900,
            },
            milliseconds: 140,
        };
        assert_eq!(t.at(0), t.from);
        assert_eq!(t.at(140), t.to);
        assert_eq!(t.at(1000), t.to);
        let mut old = t.from;
        for ms in 1..140 {
            let r = t.at(ms);
            assert!(
                r.x >= old.x
                    && r.x <= t.to.x
                    && r.y >= old.y
                    && r.y <= t.to.y
                    && r.w >= old.w
                    && r.w <= t.to.w
                    && r.h >= old.h
                    && r.h <= t.to.h
            );
            old = r;
        }
    }
}
