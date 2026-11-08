//! Session-local binary tiling tree. Window identities never enter portable profiles.
use crate::layout::Rect;
#[derive(Clone, Debug)]
enum Node {
    Leaf(isize),
    Split {
        vertical: bool,
        ratio: f64,
        a: Box<Node>,
        b: Box<Node>,
    },
}
#[derive(Clone, Default, Debug)]
pub struct Tree(Option<Node>);
impl Node {
    fn prune(self, keys: &[isize]) -> Option<Self> {
        match self {
            Self::Leaf(key) => keys.contains(&key).then_some(Self::Leaf(key)),
            Self::Split {
                vertical,
                ratio,
                a,
                b,
            } => match (a.prune(keys), b.prune(keys)) {
                (Some(a), Some(b)) => Some(Self::Split {
                    vertical,
                    ratio,
                    a: Box::new(a),
                    b: Box::new(b),
                }),
                (a, b) => a.or(b),
            },
        }
    }
    fn minimum(&self, gap: i32) -> (i32, i32) {
        match self {
            Self::Leaf(_) => (240, 140),
            Self::Split { vertical, a, b, .. } => {
                let (aw, ah) = a.minimum(gap);
                let (bw, bh) = b.minimum(gap);
                if *vertical {
                    (aw + bw + gap, ah.max(bh))
                } else {
                    (aw.max(bw), ah + bh + gap)
                }
            }
        }
    }
    fn render(&self, area: Rect, gap: i32, out: &mut Vec<(isize, Rect)>) -> bool {
        match self {
            Self::Leaf(key) => {
                out.push((*key, area));
                true
            }
            Self::Split {
                vertical,
                ratio,
                a,
                b,
            } => {
                let (amin, bmin, length) = if *vertical {
                    (a.minimum(gap).0, b.minimum(gap).0, area.w)
                } else {
                    (a.minimum(gap).1, b.minimum(gap).1, area.h)
                };
                let available = length - gap;
                if available < amin + bmin {
                    return false;
                }
                let cut = ((available as f64 * ratio).round() as i32).clamp(amin, available - bmin);
                let mut left = area;
                let mut right = area;
                if *vertical {
                    left.w = cut;
                    right.x += cut + gap;
                    right.w = available - cut;
                } else {
                    left.h = cut;
                    right.y += cut + gap;
                    right.h = available - cut;
                }
                a.render(left, gap, out) && b.render(right, gap, out)
            }
        }
    }
    fn insert(&mut self, target: isize, key: isize, vertical: bool) -> bool {
        match self {
            Self::Leaf(old) if *old == target => {
                *self = Self::Split {
                    vertical,
                    ratio: 0.5,
                    a: Box::new(Self::Leaf(*old)),
                    b: Box::new(Self::Leaf(key)),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { a, b, .. } => {
                a.insert(target, key, vertical) || b.insert(target, key, vertical)
            }
        }
    }
    fn contains(&self, key: isize) -> bool {
        match self {
            Self::Leaf(k) => *k == key,
            Self::Split { a, b, .. } => a.contains(key) || b.contains(key),
        }
    }
    fn parent_change(&mut self, key: isize, flip: bool, delta: f64) -> bool {
        match self {
            Self::Leaf(_) => false,
            Self::Split {
                vertical,
                ratio,
                a,
                b,
            } => {
                if a.contains(key) && a.parent_change(key, flip, delta)
                    || b.contains(key) && b.parent_change(key, flip, delta)
                {
                    return true;
                }
                if matches!(**a,Self::Leaf(k) if k==key) || matches!(**b,Self::Leaf(k) if k==key) {
                    if flip {
                        *vertical = !*vertical;
                    } else {
                        *ratio = (*ratio + delta).clamp(0.1, 0.9);
                    }
                    true
                } else {
                    false
                }
            }
        }
    }
    fn swap(&mut self, x: isize, y: isize) {
        match self {
            Self::Leaf(k) => {
                if *k == x {
                    *k = y;
                } else if *k == y {
                    *k = x;
                }
            }
            Self::Split { a, b, .. } => {
                a.swap(x, y);
                b.swap(x, y);
            }
        }
    }
    fn adopt(&mut self, rects: &[(isize, Rect)]) -> Option<Rect> {
        match self {
            Self::Leaf(k) => rects.iter().find(|(key, _)| key == k).map(|(_, r)| *r),
            Self::Split {
                vertical,
                ratio,
                a,
                b,
            } => {
                let x = a.adopt(rects)?;
                let y = b.adopt(rects)?;
                *ratio = if *vertical {
                    x.w as f64 / (x.w + y.w) as f64
                } else {
                    x.h as f64 / (x.h + y.h) as f64
                };
                Some(Rect {
                    x: x.x.min(y.x),
                    y: x.y.min(y.y),
                    w: (x.x + x.w).max(y.x + y.w) - x.x.min(y.x),
                    h: (x.y + x.h).max(y.y + y.h) - x.y.min(y.y),
                })
            }
        }
    }
}
impl Tree {
    pub fn rects(&self, area: Rect, gap: i32) -> Vec<(isize, Rect)> {
        let Some(inner) = crate::layout::tile(area, 1, gap, 0.5).first().copied() else {
            return vec![];
        };
        let mut out = vec![];
        if let Some(root) = &self.0 {
            if !root.render(inner, gap, &mut out) {
                return vec![];
            }
        }
        out
    }
    pub fn sync(&mut self, keys: &[isize], focus: Option<isize>, area: Rect, gap: i32) {
        self.0 = self.0.take().and_then(|node| node.prune(keys));
        // If the work area shrank, rebuild rather than overlap windows or invent tiny panes.
        if self.0.is_some() && self.rects(area, gap).is_empty() {
            self.0 = None;
        }
        for &key in keys {
            if self.0.as_ref().is_some_and(|node| node.contains(key)) {
                continue;
            }
            if self.0.is_none() {
                self.0 = Some(Node::Leaf(key));
                continue;
            }
            let mut candidates = self.rects(area, gap);
            let focus = focus
                .filter(|key| candidates.iter().any(|(k, _)| k == key))
                .or_else(|| candidates.last().map(|(k, _)| *k));
            candidates.sort_by_key(|(k, r)| {
                (
                    Some(*k) != focus,
                    std::cmp::Reverse(i64::from(r.w) * i64::from(r.h)),
                )
            });
            for (target, r) in candidates {
                let vertical = if r.w >= r.h && r.w >= 480 + gap {
                    true
                } else if r.h >= 280 + gap {
                    false
                } else if r.w >= 480 + gap {
                    true
                } else {
                    continue;
                };
                self.0.as_mut().unwrap().insert(target, key, vertical);
                break;
            }
        }
    }
    pub fn change(&mut self, key: isize, flip: bool, delta: f64, area: Rect, gap: i32) -> bool {
        let mut next = self.clone();
        if !next
            .0
            .as_mut()
            .is_some_and(|node| node.parent_change(key, flip, delta))
            || next.rects(area, gap).is_empty()
        {
            return false;
        }
        *self = next;
        true
    }
    pub fn swap(&mut self, x: isize, y: isize) {
        if let Some(root) = &mut self.0 {
            root.swap(x, y);
        }
    }
    pub fn adopt(&mut self, rects: &[(isize, Rect)]) {
        if let Some(root) = &mut self.0 {
            root.adopt(rects);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn area() -> Rect {
        Rect {
            x: -1920,
            y: 40,
            w: 1920,
            h: 1040,
        }
    }
    #[test]
    fn splits_focused_pane_retains_siblings_and_collapses_closed_windows() {
        let mut t = Tree::default();
        t.sync(&[1, 2], None, area(), 10);
        let first = t.rects(area(), 10);
        t.sync(&[1, 2, 3], Some(2), area(), 10);
        let r = t.rects(area(), 10);
        assert_eq!(r[0], first[0]);
        assert_eq!(r.len(), 3);
        assert_eq!(r[1].1.x, r[2].1.x);
        assert!(r[2].1.y > r[1].1.y);
        t.sync(&[1, 2], None, area(), 10);
        assert_eq!(t.rects(area(), 10), first);
    }
    #[test]
    fn flip_ratios_mouse_sizes_and_swaps_are_retained() {
        let mut t = Tree::default();
        t.sync(&[1, 2], None, area(), 10);
        assert!(t.change(1, true, 0., area(), 10));
        let r = t.rects(area(), 10);
        assert_eq!(r[0].1.x, r[1].1.x);
        assert!(t.change(1, false, 0.1, area(), 10));
        assert!(t.rects(area(), 10)[0].1.h > r[0].1.h);
        t.swap(1, 2);
        assert_eq!(t.rects(area(), 10)[0].0, 2);
        let mut resized = t.rects(area(), 10);
        resized[0].1.h += 30;
        resized[1].1.y += 30;
        resized[1].1.h -= 30;
        t.adopt(&resized);
        assert_eq!(t.rects(area(), 10), resized);
    }
    #[test]
    fn overflow_remains_outside_tree_and_small_screens_do_not_overlap() {
        let mut t = Tree::default();
        let small = Rect {
            x: 0,
            y: 0,
            w: 600,
            h: 400,
        };
        t.sync(&(1..=30).collect::<Vec<_>>(), None, small, 10);
        let r = t.rects(small, 10);
        assert!(r.len() < 30);
        for (_, r) in r {
            assert!(r.w >= 240 && r.h >= 140);
        }
    }
}
