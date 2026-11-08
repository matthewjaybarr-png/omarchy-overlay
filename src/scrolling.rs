//! A monitor-local viewport over an ordered horizontal strip of windows.
use crate::layout::Rect;
#[derive(Clone, Debug)]
pub struct Viewport {
    pub start: usize,
    pub visible: Vec<(usize, Rect)>,
}
pub fn viewport(
    area: Rect,
    count: usize,
    focus: usize,
    previous: usize,
    columns: usize,
    gap: i32,
) -> Viewport {
    if count == 0 || area.w <= 0 || area.h <= 0 {
        return Viewport {
            start: 0,
            visible: vec![],
        };
    }
    let columns = columns.clamp(1, 4).min(count);
    let focus = focus.min(count - 1);
    let mut start = previous.min(count - columns);
    if focus < start {
        start = focus;
    } else if focus >= start + columns {
        start = focus + 1 - columns;
    }
    let rects = crate::layout::arrange(area, columns, gap, 0.5, "columns");
    Viewport {
        start,
        visible: rects
            .into_iter()
            .enumerate()
            .map(|(i, r)| (start + i, r))
            .collect(),
    }
}
#[cfg(windows)]
#[derive(Clone, Copy, Default)]
pub struct State {
    pub active: isize,
    pub start: usize,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moves_one_column_at_an_edge_and_keeps_visible_focus_stable() {
        let area = Rect {
            x: 0,
            y: 30,
            w: 1920,
            h: 1050,
        };
        assert_eq!(viewport(area, 5, 1, 0, 2, 8).start, 0);
        let view = viewport(area, 5, 2, 0, 2, 8);
        assert_eq!(view.start, 1);
        assert_eq!(
            view.visible.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(viewport(area, 5, 0, 3, 2, 8).start, 0);
    }
    #[test]
    fn four_monitor_viewports_never_spill_and_handle_removals() {
        for area in [
            Rect {
                x: -1920,
                y: 0,
                w: 1920,
                h: 1040,
            },
            Rect {
                x: 0,
                y: 40,
                w: 2560,
                h: 1400,
            },
            Rect {
                x: 2560,
                y: -700,
                w: 1200,
                h: 1800,
            },
            Rect {
                x: 0,
                y: 1480,
                w: 1600,
                h: 860,
            },
        ] {
            for columns in 1..=4 {
                for focus in 0..8 {
                    let view = viewport(area, 8, focus, 7, columns, 8);
                    assert!(view.visible.iter().any(|(i, _)| *i == focus));
                    for (_, r) in view.visible {
                        assert!(
                            r.x >= area.x
                                && r.y >= area.y
                                && r.x + r.w <= area.x + area.w
                                && r.y + r.h <= area.y + area.h
                        );
                    }
                }
            }
            assert_eq!(viewport(area, 1, 99, 99, 4, 8).visible.len(), 1);
            assert!(viewport(area, 0, 0, 0, 2, 8).visible.is_empty());
        }
    }
}
