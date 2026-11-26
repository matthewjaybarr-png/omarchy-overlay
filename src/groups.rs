//! Omarchy's window groups: grouped windows share one tile and one shows at a time.
//! The others are minimized with the scrolling layout's hidden mark, so pause, exit and
//! crash recovery bring them back the way they bring back off-view scrolling windows.
use super::*;

pub(super) struct Group {
    pub(super) members: Vec<isize>,
    pub(super) shown: usize,
    /// When the shown window last changed: a window just minimized may still read as open.
    switched: std::time::Instant,
}

impl App {
    fn group_of(&self, key: isize) -> Option<usize> {
        self.groups.iter().position(|g| g.members.contains(&key))
    }
    /// A grouped window that waits behind the shown one.
    pub(super) fn group_hidden(&self, key: isize) -> bool {
        self.groups
            .iter()
            .any(|g| g.members.iter().enumerate().any(|(i, k)| *k == key && i != g.shown))
    }
    /// Shows member `next` of group `g` in the tile of the one shown now.
    unsafe fn show_member(&mut self, g: usize, next: usize) {
        let group = &self.groups[g];
        let (old, new) = (group.members[group.shown], group.members[next]);
        if old == new {
            return;
        }
        if IsWindow(old as HWND) != 0 && !self.hide_member(old as HWND) {
            return;
        }
        let group = &mut self.groups[g];
        group.shown = next;
        group.switched = std::time::Instant::now();
        if let Some(slot) = self.order.iter().position(|k| *k == old) {
            self.order[slot] = new;
        }
        for tree in self.dwindle.values_mut() {
            tree.swap(old, new);
        }
        let hwnd = new as HWND;
        Self::unhide(hwnd);
        SetForegroundWindow(hwnd);
        self.dirty = true;
    }
    unsafe fn hide_member(&mut self, hwnd: HWND) -> bool {
        if !self.snapshot(hwnd) {
            return false;
        }
        let marker = self.saved.get(&(hwnd as isize)).map_or(0, |saved| saved.marker);
        if !scrolling_runtime::owned(hwnd)
            && SetPropW(hwnd, wide(scrolling_runtime::HIDDEN).as_ptr(), marker as HANDLE) == 0
        {
            return false;
        }
        ShowWindowAsync(hwnd, SW_SHOWMINNOACTIVE);
        true
    }
    unsafe fn unhide(hwnd: HWND) {
        if IsIconic(hwnd) != 0 {
            // Synchronous where possible: the tiling pass that follows reads the window.
            if responsive(hwnd) {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            } else {
                ShowWindowAsync(hwnd, SW_SHOWNOACTIVATE);
            }
        }
        RemovePropW(hwnd, wide(scrolling_runtime::HIDDEN).as_ptr());
    }
    unsafe fn feedback_for(&self, g: usize) {
        let group = &self.groups[g];
        self.show_feedback(&format!("Group {}/{}", group.shown + 1, group.members.len()));
    }
    /// Super+G: makes the focused window a group, or breaks its group up into tiles.
    pub(super) unsafe fn toggle_group(&mut self, hwnd: HWND) {
        let key = hwnd as isize;
        if let Some(g) = self.group_of(key) {
            for member in self.groups.remove(g).members {
                Self::unhide(member as HWND);
            }
            self.show_feedback("Ungrouped");
        } else if self.movable(hwnd) && self.order.contains(&key) {
            self.groups.push(Group { members: vec![key], shown: 0, switched: std::time::Instant::now() });
            self.show_feedback("Grouped");
        } else {
            return;
        }
        self.dirty = true;
        self.tick();
    }
    /// Super+Alt+G: takes the window out of its group into a tile of its own.
    pub(super) unsafe fn leave_group(&mut self, hwnd: HWND) {
        let key = hwnd as isize;
        let Some(g) = self.group_of(key) else {
            return;
        };
        let at = self.groups[g].members.iter().position(|k| *k == key).unwrap_or(0);
        if self.groups[g].members.len() > 1 && at == self.groups[g].shown {
            // The next member takes the tile; the window that leaves enters as a new tile.
            let next = (at + 1) % self.groups[g].members.len();
            let next_key = self.groups[g].members[next];
            if let Some(slot) = self.order.iter().position(|k| *k == key) {
                self.order[slot] = next_key;
            }
            for tree in self.dwindle.values_mut() {
                tree.swap(key, next_key);
            }
            Self::unhide(next_key as HWND);
            SetForegroundWindow(hwnd);
        }
        let group = &mut self.groups[g];
        group.members.remove(at);
        let shown_key = if at == group.shown {
            group.members.get(at % group.members.len().max(1)).copied()
        } else {
            group.members.get(group.shown - usize::from(at < group.shown)).copied()
        };
        group.shown = shown_key.and_then(|k| group.members.iter().position(|m| *m == k)).unwrap_or(0);
        group.switched = std::time::Instant::now();
        if group.members.is_empty() {
            self.groups.remove(g);
        }
        Self::unhide(hwnd);
        self.dirty = true;
        self.tick();
    }
    /// Super+Alt+arrows: moves the window into the group of the tile in that direction,
    /// making that tile a group first when it is not one.
    pub(super) unsafe fn move_into_group(&mut self, hwnd: HWND, dx: i32, dy: i32) {
        let key = hwnd as isize;
        if !self.order.contains(&key) {
            return;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let tiles: Vec<isize> = self
            .order
            .iter()
            .copied()
            .filter(|k| {
                IsIconic(*k as HWND) == 0
                    && MonitorFromWindow(*k as HWND, MONITOR_DEFAULTTONEAREST) == monitor
            })
            .collect();
        let rects: Vec<Rect> = tiles
            .iter()
            .map(|k| {
                let mut r: RECT = zeroed();
                GetWindowRect(*k as HWND, &mut r);
                Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
            })
            .collect();
        let Some(index) = tiles.iter().position(|k| *k == key) else {
            return;
        };
        let Some(target) = layout::directional(&rects, index, dx, dy).map(|i| tiles[i]) else {
            return;
        };
        if self.group_of(key).is_some() {
            self.leave_group(hwnd);
        }
        let g = match self.group_of(target) {
            Some(g) => g,
            None => {
                self.groups.push(Group { members: vec![target], shown: 0, switched: std::time::Instant::now() });
                self.groups.len() - 1
            }
        };
        self.order.retain(|k| *k != key);
        self.groups[g].members.push(key);
        // The window keeps showing and takes the target's tile.
        let group = &self.groups[g];
        let (old, new) = (group.members[group.shown], group.members.len() - 1);
        if !self.hide_member(old as HWND) {
            self.groups[g].members.pop();
            self.dirty = true;
            self.tick();
            return;
        }
        if let Some(slot) = self.order.iter().position(|k| *k == old) {
            self.order[slot] = key;
        }
        for tree in self.dwindle.values_mut() {
            tree.swap(old, key);
        }
        let group = &mut self.groups[g];
        group.shown = new;
        group.switched = std::time::Instant::now();
        SetForegroundWindow(hwnd);
        self.feedback_for(g);
        self.dirty = true;
        self.tick();
    }
    /// Super+Alt+Tab and Super+Ctrl+arrows: shows the next or previous window of the group.
    /// Omarchy's Super+Alt+1..5: shows window `index` of the focused window's group.
    pub(super) unsafe fn show_group_window(&mut self, hwnd: HWND, index: usize) {
        let Some(g) = self.group_of(hwnd as isize) else {
            return;
        };
        if index < self.groups[g].members.len() {
            self.show_member(g, index);
            self.feedback_for(g);
            self.tick();
        }
    }
    pub(super) unsafe fn cycle_group(&mut self, hwnd: HWND, step: i32) {
        let Some(g) = self.group_of(hwnd as isize) else {
            return;
        };
        let len = self.groups[g].members.len() as i32;
        if len < 2 {
            return;
        }
        let next = (self.groups[g].shown as i32 + step).rem_euclid(len) as usize;
        self.show_member(g, next);
        self.feedback_for(g);
        self.tick();
    }
    /// Shows a returning group's window that took over, while parked, from one that closed.
    pub(super) unsafe fn show_returned_groups(&self, back: &[isize]) {
        for group in &self.groups {
            let hwnd = group.members[group.shown] as HWND;
            if back.contains(&(hwnd as isize))
                && IsIconic(hwnd) != 0
                && !GetPropW(hwnd, wide(scrolling_runtime::HIDDEN).as_ptr()).is_null()
            {
                Self::unhide(hwnd);
            }
        }
    }
    /// Run at the start of every tiling pass: drops closed windows, shows the next member
    /// when the shown one closes, and follows a hidden member the user opened (taskbar, Alt+Tab).
    pub(super) unsafe fn sync_groups(&mut self) {
        let mut g = 0;
        while g < self.groups.len() {
            let shown_key = self.groups[g].members[self.groups[g].shown];
            if IsWindow(shown_key as HWND) == 0 {
                let group = &mut self.groups[g];
                group.members.remove(group.shown);
                if group.members.is_empty() {
                    self.groups.remove(g);
                    continue;
                }
                group.shown %= group.members.len();
                let new = group.members[group.shown];
                if let Some(slot) = self.order.iter().position(|k| *k == shown_key) {
                    self.order[slot] = new;
                }
                for tree in self.dwindle.values_mut() {
                    tree.swap(shown_key, new);
                }
                // A group parked on another workspace shows its next window when it comes back.
                if !self.parked.contains_key(&new) {
                    Self::unhide(new as HWND);
                }
                self.dirty = true;
            }
            let group = &mut self.groups[g];
            let shown = group.shown;
            let current = group.members[shown];
            group.members.retain(|&k| k == current || IsWindow(k as HWND) != 0);
            group.shown = group.members.iter().position(|k| *k == current).unwrap_or(0);
            let settled = group.switched.elapsed() > std::time::Duration::from_millis(1500);
            let opened = group.members.iter().position(|&k| {
                k != current && !self.parked.contains_key(&k) && IsIconic(k as HWND) == 0
            });
            if let (true, Some(next)) = (settled, opened) {
                RemovePropW(group.members[next] as HWND, wide(scrolling_runtime::HIDDEN).as_ptr());
                self.show_member(g, next);
            } else if IsIconic(current as HWND) == 0 {
                RemovePropW(current as HWND, wide(scrolling_runtime::HIDDEN).as_ptr());
            }
            g += 1;
        }
    }
}
