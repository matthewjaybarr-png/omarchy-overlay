#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Shifts the work area so layouts, which put `gap` at their edges, end `outer_gap` from the monitor edge.
pub fn outer(area: Rect, gap: i32, outer_gap: i32) -> Rect {
    let d = outer_gap - gap;
    Rect {
        x: area.x + d,
        y: area.y + d,
        w: (area.w - 2 * d).max(1),
        h: (area.h - 2 * d).max(1),
    }
}

/// The first window is the main pane; the remainder share a vertical stack.
pub fn tile(area: Rect, count: usize, gap: i32, ratio: f64) -> Vec<Rect> {
    if count == 0 || area.w < 1 || area.h < 1 {
        return vec![];
    }
    let gap = gap.clamp(0, (area.w.min(area.h) / 10).max(0));
    let inner = Rect {
        x: area.x + gap,
        y: area.y + gap,
        w: (area.w - 2 * gap).max(1),
        h: (area.h - 2 * gap).max(1),
    };
    if count == 1 {
        return vec![inner];
    }
    let left = ((inner.w - gap) as f64 * ratio.clamp(0.25, 0.75)).round() as i32;
    let mut result = vec![Rect {
        w: left.max(1),
        ..inner
    }];
    result.extend(column(
        Rect {
            x: inner.x + left + gap,
            w: (inner.w - left - gap).max(1),
            ..inner
        },
        count - 1,
        gap,
    ));
    result
}

/// Splits one column into `n` rows of equal height.
fn column(space: Rect, n: usize, gap: i32) -> Vec<Rect> {
    let n = n as i32;
    if n < 1 {
        return vec![];
    }
    let gap = gap.min((space.h / n - 1).max(0));
    let available = (space.h - gap * (n - 1)).max(n);
    (0..n)
        .map(|i| {
            let start = available * i / n;
            Rect {
                y: space.y + start + i * gap,
                h: available * (i + 1) / n - start,
                ..space
            }
        })
        .collect()
}

/// Main pane in the middle, the rest shared between a right and a left column
/// (second window right, third left, and so on). Two windows are main and stack.
fn centered(area: Rect, inner: Rect, count: usize, gap: i32, ratio: f64) -> Vec<Rect> {
    if count < 3 {
        return tile(area, count, gap, ratio);
    }
    let main = ((inner.w - 2 * gap) as f64 * ratio.clamp(0.25, 0.75)).round() as i32;
    let side = ((inner.w - 2 * gap - main) / 2).max(1);
    let main = (inner.w - 2 * gap - 2 * side).max(1);
    let right = column(
        Rect {
            x: inner.x + side + main + 2 * gap,
            w: side,
            ..inner
        },
        count / 2,
        gap,
    );
    let left = column(Rect { w: side, ..inner }, (count - 1) / 2, gap);
    let mut result = vec![Rect {
        x: inner.x + side + gap,
        w: main,
        ..inner
    }];
    for i in 0..count - 1 {
        result.push(if i % 2 == 0 {
            right[i / 2]
        } else {
            left[i / 2]
        });
    }
    result
}

pub fn directional(rects: &[Rect], current: usize, dx: i32, dy: i32) -> Option<usize> {
    let a = rects.get(current)?;
    let (ax, ay) = (a.x + a.w / 2, a.y + a.h / 2);
    rects
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != current)
        .filter_map(|(i, b)| {
            let (x, y) = (b.x + b.w / 2 - ax, b.y + b.h / 2 - ay);
            let forward = x * dx + y * dy;
            if forward <= 0 {
                return None;
            }
            let side = (x * dy - y * dx).abs();
            Some((i, i64::from(forward) + i64::from(side) * 3))
        })
        .min_by_key(|(_, score)| *score)
        .map(|(i, _)| i)
}

/// Choose near 16:10 cells while respecting the same usable pane sizes as other layouts.
fn grid_columns(inner: Rect, count: usize, gap: i32) -> usize {
    (1..=count)
        .filter(|cols| {
            let rows = count.div_ceil(*cols);
            (inner.w - gap * (*cols as i32 - 1)) / *cols as i32 >= 320.min(inner.w)
                && (inner.h - gap * (rows as i32 - 1)) / rows as i32 >= 180.min(inner.h)
        })
        .min_by(|a, b| {
            let score = |cols: usize| {
                let rows = count.div_ceil(cols);
                let w = (inner.w - gap * (cols as i32 - 1)).max(1) as f64 / cols as f64;
                let h = (inner.h - gap * (rows as i32 - 1)).max(1) as f64 / rows as f64;
                (w / h / 1.6).ln().abs() + (cols * rows - count) as f64 / count as f64 * 0.4
            };
            score(*a).total_cmp(&score(*b))
        })
        .unwrap_or(1)
}
pub fn grid_capacity(area: Rect, gap: i32) -> usize {
    (((area.w - 2 * gap) / (320 + gap).max(1)).max(1)
        * ((area.h - 2 * gap) / (180 + gap).max(1)).max(1)) as usize
}
pub fn arrange(area: Rect, count: usize, gap: i32, ratio: f64, mode: &str) -> Vec<Rect> {
    if mode == "main-stack" {
        return tile(area, count, gap, ratio);
    }
    let Some(inner) = tile(area, 1, gap, ratio).first().copied() else {
        return vec![];
    };
    if count == 0 {
        return vec![];
    }
    if mode == "monocle" {
        // Every window fills the monitor; the focused one is on top.
        return vec![inner; count];
    }
    if mode == "centered" {
        return centered(area, inner, count, gap, ratio);
    }
    if mode == "grid" {
        let cols = grid_columns(inner, count, gap);
        let rows = count.div_ceil(cols);
        let vertical_gap = gap.min((inner.h / rows as i32 - 1).max(0));
        let height = (inner.h - vertical_gap * (rows as i32 - 1)).max(rows as i32);
        return (0..count)
            .map(|index| {
                let row = index / cols;
                let col = index % cols;
                let columns = (count - row * cols).min(cols);
                let horizontal_gap = gap.min((inner.w / columns as i32 - 1).max(0));
                let width = (inner.w - horizontal_gap * (columns as i32 - 1)).max(columns as i32);
                let left = width * col as i32 / columns as i32;
                let right = width * (col + 1) as i32 / columns as i32;
                let top = height * row as i32 / rows as i32;
                let bottom = height * (row + 1) as i32 / rows as i32;
                Rect {
                    x: inner.x + left + horizontal_gap * col as i32,
                    y: inner.y + top + vertical_gap * row as i32,
                    w: right - left,
                    h: bottom - top,
                }
            })
            .collect();
    }
    let horizontal = mode == "columns";
    let length = if horizontal { inner.w } else { inner.h };
    let n = count as i32;
    let gap = gap.clamp(0, (length / n - 1).max(0));
    let available = (length - gap * (n - 1)).max(n);
    (0..n)
        .map(|i| {
            let start = available * i / n;
            let end = available * (i + 1) / n;
            if horizontal {
                Rect {
                    x: inner.x + start + i * gap,
                    w: end - start,
                    ..inner
                }
            } else {
                Rect {
                    y: inner.y + start + i * gap,
                    h: end - start,
                    ..inner
                }
            }
        })
        .collect()
}
/// Where the floating layout first puts a window, as Omarchy's floating workspace
/// mode does: min(1100, 64%) x min(760, 68%) of the area, centred, then stepped
/// diagonally by 24 px in a cycle of five so windows floated together do not stack
/// exactly. `index` counts the windows already placed on that monitor. Sizes are
/// Omarchy's logical pixels, so `dpi` scales the caps and the step: at 125% the cap is
/// 1375 physical pixels, as Hyprland's 1100 is at scale 1.25.
pub fn floating(area: Rect, index: usize, dpi: u32) -> Rect {
    let scale = |v: i32| v * dpi.max(96) as i32 / 96;
    let w = scale(1100).min(area.w * 64 / 100).max(1);
    let h = scale(760).min(area.h * 68 / 100).max(1);
    let step = scale(((index + 1) % 5) as i32 * 24 - 48);
    Rect {
        x: area.x + (area.w - w) / 2 + step,
        y: area.y + (area.h - h) / 2 + step,
        w,
        h,
    }
}
/// Omarchy's single-window aspect ratio (1:1): a lone tile as the largest square that
/// fits the area, centred, so one window doesn't stretch across a wide screen.
pub fn square(area: Rect) -> Rect {
    let side = area.w.min(area.h).max(1);
    Rect {
        x: area.x + (area.w - side) / 2,
        y: area.y + (area.h - side) / 2,
        w: side,
        h: side,
    }
}
/// Hyprland's pseudotile: a window keeps its own size, shrunk to fit, centred in its tile.
pub fn pseudo(tile: Rect, width: i32, height: i32) -> Rect {
    let (w, h) = (width.clamp(1, tile.w.max(1)), height.clamp(1, tile.h.max(1)));
    Rect { x: tile.x + (tile.w - w) / 2, y: tile.y + (tile.h - h) / 2, w, h }
}
pub fn transfer(window: Rect, source: Rect, target: Rect) -> Rect {
    let w = window.w.min(target.w).max(1);
    let h = window.h.min(target.h).max(1);
    let relative_x = if source.w > window.w {
        f64::from(window.x - source.x) / f64::from(source.w - window.w)
    } else {
        0.5
    };
    let relative_y = if source.h > window.h {
        f64::from(window.y - source.y) / f64::from(source.h - window.h)
    } else {
        0.5
    };
    Rect {
        x: target.x + ((target.w - w) as f64 * relative_x.clamp(0.0, 1.0)).round() as i32,
        y: target.y + ((target.h - h) as f64 * relative_y.clamp(0.0, 1.0)).round() as i32,
        w,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outer_gap_sets_monitor_edges_and_gap_sets_spacing() {
        let area = Rect {
            x: 0,
            y: 0,
            w: 1000,
            h: 800,
        };
        let r = tile(outer(area, 5, 10), 2, 5, 0.5);
        assert_eq!((r[0].x, r[0].y), (10, 10));
        assert_eq!(r[1].x - (r[0].x + r[0].w), 5);
        assert_eq!((r[1].x + r[1].w, r[0].y + r[0].h), (990, 790));
        assert_eq!(outer(area, 10, 0).x, -10);
    }
    #[test]
    fn grid_fits_irregular_screens_without_overlap_and_fills_last_row() {
        for area in [
            Rect {
                x: -1920,
                y: -100,
                w: 1920,
                h: 1080,
            },
            Rect {
                x: 2400,
                y: 0,
                w: 1080,
                h: 1880,
            },
            Rect {
                x: 0,
                y: 0,
                w: 3440,
                h: 1400,
            },
        ] {
            for count in 1..=grid_capacity(area, 10).min(16) {
                let rects = arrange(area, count, 10, 0.55, "grid");
                assert_eq!(rects.len(), count);
                for (i, a) in rects.iter().enumerate() {
                    assert!(a.w >= 320 && a.h >= 180);
                    assert!(
                        a.x >= area.x
                            && a.y >= area.y
                            && a.x + a.w <= area.x + area.w
                            && a.y + a.h <= area.y + area.h
                    );
                    for b in rects.iter().skip(i + 1) {
                        assert!(
                            a.x + a.w <= b.x
                                || b.x + b.w <= a.x
                                || a.y + a.h <= b.y
                                || b.y + b.h <= a.y
                        );
                    }
                }
                let last = rects.last().unwrap();
                assert_eq!(last.x + last.w, area.x + area.w - 10);
            }
        }
        let r = arrange(
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
            4,
            10,
            0.55,
            "grid",
        );
        assert_eq!(r[0].y, r[1].y);
        assert!(r[2].y > r[0].y);
    }
    #[test]
    fn layouts_fit_four_irregular_monitor_work_areas() {
        let monitors = [
            Rect {
                x: -1920,
                y: 40,
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
                y: -500,
                w: 1080,
                h: 1880,
            },
            Rect {
                x: 0,
                y: -1080,
                w: 1920,
                h: 1040,
            },
        ];
        for area in monitors {
            for mode in ["main-stack", "columns", "rows"] {
                for n in 1..=4 {
                    let panes = arrange(area, n, 12, 0.55, mode);
                    assert_eq!(panes.len(), n);
                    for (i, r) in panes.iter().enumerate() {
                        assert!(
                            r.x >= area.x
                                && r.y >= area.y
                                && r.x + r.w <= area.x + area.w
                                && r.y + r.h <= area.y + area.h
                        );
                        assert!(r.w > 0 && r.h > 0);
                        for s in &panes[i + 1..] {
                            assert!(
                                r.x + r.w <= s.x
                                    || s.x + s.w <= r.x
                                    || r.y + r.h <= s.y
                                    || s.y + s.h <= r.y
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn transfers_to_negative_and_smaller_monitors_without_losing_windows() {
        let source = Rect {
            x: 0,
            y: 40,
            w: 2560,
            h: 1400,
        };
        let target = Rect {
            x: -1280,
            y: -300,
            w: 1280,
            h: 984,
        };
        for window in [
            Rect {
                x: 1400,
                y: 800,
                w: 900,
                h: 600,
            },
            Rect {
                x: 0,
                y: 40,
                w: 2560,
                h: 1400,
            },
        ] {
            let moved = transfer(window, source, target);
            assert!(moved.x >= target.x && moved.y >= target.y);
            assert!(
                moved.x + moved.w <= target.x + target.w
                    && moved.y + moved.h <= target.y + target.h
            );
        }
    }
    #[test]
    fn preserves_negative_monitor_coordinates_and_work_area() {
        let area = Rect {
            x: -1920,
            y: 40,
            w: 1920,
            h: 1040,
        };
        for count in 1..12 {
            let rects = tile(area, count, 10, 0.55);
            assert_eq!(rects.len(), count);
            for (i, r) in rects.iter().enumerate() {
                assert!(r.w > 0 && r.h > 0);
                assert!(r.x >= area.x && r.y >= area.y);
                assert!(r.x + r.w <= area.x + area.w && r.y + r.h <= area.y + area.h);
                for s in &rects[i + 1..] {
                    assert!(
                        r.x + r.w <= s.x
                            || s.x + s.w <= r.x
                            || r.y + r.h <= s.y
                            || s.y + s.h <= r.y
                    );
                }
            }
        }
    }
    #[test]
    fn navigation_follows_geometry() {
        let rects = tile(
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            },
            4,
            10,
            0.55,
        );
        assert_eq!(directional(&rects, 0, 1, 0), Some(2));
        assert_eq!(directional(&rects, 2, 0, 1), Some(3));
        assert_eq!(directional(&rects, 2, -1, 0), Some(0));
        assert_eq!(directional(&rects, 0, -1, 0), None);
    }
    #[test]
    fn centered_and_monocle_fill_the_area_without_overlap() {
        let area = Rect {
            x: -3440,
            y: 0,
            w: 3440,
            h: 1400,
        };
        for count in 1..=7 {
            let rects = arrange(area, count, 10, 0.5, "centered");
            assert_eq!(rects.len(), count);
            for (i, a) in rects.iter().enumerate() {
                assert!(
                    a.x >= area.x + 10 && a.x + a.w <= area.x + area.w - 10,
                    "{count} {a:?}"
                );
                assert!(a.y >= area.y + 10 && a.y + a.h <= area.y + area.h - 10);
                for b in &rects[i + 1..] {
                    assert!(
                        a.x + a.w <= b.x
                            || b.x + b.w <= a.x
                            || a.y + a.h <= b.y
                            || b.y + b.h <= a.y,
                        "{count}: {a:?} overlaps {b:?}"
                    );
                }
            }
            if count >= 3 {
                // Main is in the middle: second window to its right, third to its left.
                assert!(rects[1].x > rects[0].x && rects[2].x < rects[0].x);
                assert_eq!(
                    rects[0].x - area.x,
                    area.x + area.w - rects[0].x - rects[0].w
                );
            }
            let all = arrange(area, count, 10, 0.5, "monocle");
            assert_eq!(all.len(), count);
            assert!(all.iter().all(|r| *r
                == Rect {
                    x: -3430,
                    y: 10,
                    w: 3420,
                    h: 1380
                }));
        }
    }
    #[test]
    fn floating_centres_a_comfortable_size_and_cascades_in_fives() {
        let area = Rect { x: -1920, y: 40, w: 1920, h: 1040 };
        let first = floating(area, 0, 96);
        assert_eq!((first.w, first.h), (1100, 707));
        // Centre of the area, moved by the first cascade step (-24, -24).
        assert_eq!(first.x, -1920 + (1920 - 1100) / 2 - 24);
        assert_eq!(first.y, 40 + (1040 - 707) / 2 - 24);
        let steps: Vec<i32> = (0..6).map(|i| floating(area, i, 96).x - floating(area, 1, 96).x).collect();
        assert_eq!(steps, vec![-24, 0, 24, 48, -48, -24]);
        let small = Rect { x: 0, y: 0, w: 800, h: 600 };
        let r = floating(small, 2, 96);
        assert_eq!((r.w, r.h), (512, 408));
        assert!(r.x >= small.x && r.x + r.w <= small.x + small.w);
        let tiny = floating(Rect { x: 0, y: 0, w: 0, h: 0 }, 0, 96);
        // 1920 x 1032 at 125%: 64% of the width (1228), as Omarchy gives at scale 1.25.
        let scaled = floating(Rect { x: 0, y: 0, w: 1920, h: 1032 }, 0, 120);
        assert_eq!((scaled.w, scaled.h), (1228, 701));
        assert!(tiny.w >= 1 && tiny.h >= 1);
    }
    #[test]
    fn pseudo_tiles_keep_their_size_inside_the_tile() {
        let tile = Rect { x: 0, y: 0, w: 1000, h: 800 };
        assert_eq!(pseudo(tile, 600, 400), Rect { x: 200, y: 200, w: 600, h: 400 });
        assert_eq!(pseudo(tile, 1600, 400), Rect { x: 0, y: 200, w: 1000, h: 400 });
    }
    #[test]
    fn a_lone_tile_can_be_a_centred_square() {
        let r = square(Rect { x: 100, y: 40, w: 3440, h: 1400 });
        assert_eq!(r, Rect { x: 100 + (3440 - 1400) / 2, y: 40, w: 1400, h: 1400 });
        let tall = square(Rect { x: 0, y: 0, w: 800, h: 1200 });
        assert_eq!((tall.w, tall.h, tall.y), (800, 800, 200));
    }
    #[test]
    fn handles_empty_and_small_work_areas() {
        assert!(tile(
            Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0
            },
            2,
            10,
            0.5
        )
        .is_empty());
        assert!(tile(
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100
            },
            0,
            10,
            0.5
        )
        .is_empty());
        let result = tile(
            Rect {
                x: 0,
                y: 0,
                w: 320,
                h: 240,
            },
            6,
            1000,
            2.0,
        );
        assert!(result
            .iter()
            .all(|r| r.w > 0 && r.h > 0 && r.y + r.h <= 240));
    }
}
