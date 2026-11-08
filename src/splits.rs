//! Adjust connected shared edges without changing the outside of a tiled group.
use crate::layout::Rect;

pub fn resize(
    rects: &[Rect],
    selected: usize,
    actual: Rect,
    gap: i32,
    minimum: (i32, i32),
) -> Vec<Rect> {
    let mut result = rects.to_vec();
    let Some(base) = rects.get(selected) else {
        return result;
    };
    for axis in [0, 1] {
        let (start, length, now_start, now_length, min_size) = if axis == 0 {
            (base.x, base.w, actual.x, actual.w, minimum.0)
        } else {
            (base.y, base.h, actual.y, actual.h, minimum.1)
        };
        if (now_length - length).abs() < 3 {
            continue;
        }
        let near_delta = now_start - start;
        let far_delta = now_start + now_length - start - length;
        let (boundary, delta) = if near_delta.abs() > far_delta.abs() {
            (start - gap, near_delta)
        } else {
            (start + length, far_delta)
        };
        let coordinates = |r: &Rect| {
            if axis == 0 {
                (r.x, r.w, r.y, r.h)
            } else {
                (r.y, r.h, r.x, r.w)
            }
        };
        let mut sides = vec![0i8; rects.len()];
        for (i, rect) in rects.iter().enumerate() {
            let (s, length, _, _) = coordinates(rect);
            if (s + length - boundary).abs() <= 1 {
                sides[i] = -1;
            } else if (s - boundary - gap).abs() <= 1 {
                sides[i] = 1;
            }
        }
        if sides[selected] == 0 {
            continue;
        }
        let mut connected = vec![false; rects.len()];
        connected[selected] = true;
        loop {
            let mut changed = false;
            for a in 0..rects.len() {
                if !connected[a] {
                    continue;
                }
                let (_, _, a_start, a_length) = coordinates(&rects[a]);
                for b in 0..rects.len() {
                    if connected[b] || sides[b] != -sides[a] {
                        continue;
                    }
                    let (_, _, b_start, b_length) = coordinates(&rects[b]);
                    if a_start.max(b_start) < (a_start + a_length).min(b_start + b_length) {
                        connected[b] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        if !connected
            .iter()
            .enumerate()
            .any(|(i, yes)| *yes && sides[i] == -sides[selected])
        {
            continue;
        }
        let mut low = i32::MIN;
        let mut high = i32::MAX;
        for (i, rect) in rects.iter().enumerate().filter(|(i, _)| connected[*i]) {
            let (_, size, _, _) = coordinates(rect);
            let minimum = min_size.min(size);
            if sides[i] < 0 {
                low = low.max(minimum - size);
            } else {
                high = high.min(size - minimum);
            }
        }
        let delta = delta.clamp(low, high);
        for (i, rect) in result.iter_mut().enumerate().filter(|(i, _)| connected[*i]) {
            if axis == 0 {
                if sides[i] < 0 {
                    rect.w += delta;
                } else {
                    rect.x += delta;
                    rect.w -= delta;
                }
            } else if sides[i] < 0 {
                rect.h += delta;
            } else {
                rect.y += delta;
                rect.h -= delta;
            }
        }
    }
    result
}
pub fn keyboard(
    rects: &[Rect],
    selected: usize,
    horizontal: bool,
    delta: i32,
    gap: i32,
    minimum: (i32, i32),
) -> Vec<Rect> {
    let Some(&base) = rects.get(selected) else {
        return rects.to_vec();
    };
    let mut actual = base;
    let far = rects.iter().any(|other| {
        if horizontal {
            (other.x - base.x - base.w - gap).abs() <= 1
                && other.y.max(base.y) < (other.y + other.h).min(base.y + base.h)
        } else {
            (other.y - base.y - base.h - gap).abs() <= 1
                && other.x.max(base.x) < (other.x + other.w).min(base.x + base.w)
        }
    });
    if horizontal {
        actual.w += delta;
        if !far {
            actual.x -= delta;
        }
    } else {
        actual.h += delta;
        if !far {
            actual.y -= delta;
        }
    }
    resize(rects, selected, actual, gap, minimum)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange;
    fn area() -> Rect {
        Rect {
            x: -1600,
            y: -100,
            w: 1600,
            h: 1000,
        }
    }
    #[test]
    fn keyboard_resize_uses_the_available_internal_edge() {
        let r = arrange(area(), 2, 10, 0.55, "columns");
        let next = keyboard(&r, 1, true, 40, 10, (240, 140));
        assert_eq!(next[1].w, r[1].w + 40);
        assert_eq!(next[1].x, r[1].x - 40);
        assert_eq!(next[0].x, r[0].x);
        assert_eq!(next[1].x + next[1].w, r[1].x + r[1].w);
    }
    #[test]
    fn main_stack_split_moves_all_stack_windows_together() {
        let rects = arrange(area(), 4, 10, 0.55, "main-stack");
        let mut actual = rects[0];
        actual.w += 100;
        let result = resize(&rects, 0, actual, 10, (240, 140));
        assert_eq!(result[0].w, rects[0].w + 100);
        for i in 1..4 {
            assert_eq!(result[i].x, rects[i].x + 100);
            assert_eq!(result[i].w, rects[i].w - 100);
            assert_eq!(result[i].y, rects[i].y);
        }
        let mut actual = rects[2];
        actual.x -= 80;
        actual.w += 80;
        assert_eq!(
            resize(&rects, 2, actual, 10, (240, 140))[0].w,
            rects[0].w - 80
        );
    }
    #[test]
    fn stack_horizontal_edge_only_adjusts_adjacent_stack_rows() {
        let rects = arrange(area(), 4, 10, 0.55, "main-stack");
        let mut actual = rects[1];
        actual.h += 60;
        let result = resize(&rects, 1, actual, 10, (240, 140));
        assert_eq!(result[0], rects[0]);
        assert_eq!(result[3], rects[3]);
        assert_eq!(result[1].h, rects[1].h + 60);
        assert_eq!(result[2].y, rects[2].y + 60);
        assert_eq!(result[2].h, rects[2].h - 60);
    }
    #[test]
    fn columns_and_rows_keep_gaps_and_outer_extent_with_minimum_sizes() {
        for mode in ["columns", "rows"] {
            let rects = arrange(area(), 3, 10, 0.55, mode);
            let mut actual = rects[0];
            if mode == "columns" {
                actual.w += 10000;
            } else {
                actual.h += 10000;
            }
            let result = resize(&rects, 0, actual, 10, (240, 140));
            assert_eq!(result[2], rects[2]);
            if mode == "columns" {
                assert_eq!(result[1].w, 240);
                assert_eq!(result[0].x + result[0].w + 10, result[1].x);
                assert_eq!(result[1].x + result[1].w, rects[1].x + rects[1].w);
            } else {
                assert_eq!(result[1].h, 140);
                assert_eq!(result[0].y + result[0].h + 10, result[1].y);
                assert_eq!(result[1].y + result[1].h, rects[1].y + rects[1].h);
            }
        }
    }
    #[test]
    fn moving_and_outer_edge_resizing_cannot_break_the_grid() {
        let rects = arrange(area(), 3, 10, 0.55, "columns");
        let mut moved = rects[1];
        moved.x += 200;
        moved.y += 100;
        assert_eq!(resize(&rects, 1, moved, 10, (240, 140)), rects);
        let mut outer = rects[0];
        outer.x -= 100;
        outer.w += 100;
        assert_eq!(resize(&rects, 0, outer, 10, (240, 140)), rects);
    }
}
