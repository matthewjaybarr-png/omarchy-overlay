//! Omarchy's groupbar (default/hypr/looknfeel.lua `group`): a row of tabs at the top of
//! a group's tile, one per window, the shown one in the accent color. Clicking a tab
//! shows that window. The tile gives up the bar's height, as Hyprland's groupbar does.
use super::*;

const CLASS: &str = "OmarchyOverlay.GroupBar";
/// Posted to the main window: wParam the bar, lParam the tab.
pub(super) const CLICK: u32 = WM_APP + 77;
/// Bar height at 96 DPI.
pub(super) const HEIGHT: i32 = 24;

struct Bar {
    hwnd: HWND,
    tabs: Vec<(String, bool)>,
}
struct State {
    owner: HWND,
    bars: Vec<Bar>,
}
thread_local! { static BARS: RefCell<Option<State>> = const { RefCell::new(None) }; }

unsafe fn register() {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        let name = wide(CLASS);
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: GetModuleHandleW(null()),
            hCursor: LoadCursorW(null_mut(), IDC_HAND),
            lpszClassName: name.as_ptr(),
            ..zeroed()
        };
        RegisterClassW(&class);
    });
}
pub(super) unsafe fn dpi_of(hwnd: HWND) -> i32 {
    GetDpiForWindow(hwnd).max(96) as i32
}
/// Shows one bar per entry: (where, tab titles with the shown one marked); hides the rest.
pub(super) unsafe fn update(owner: HWND, wanted: Vec<(Rect, Vec<(String, bool)>)>) {
    register();
    BARS.with(|cell| {
        let mut cell = cell.borrow_mut();
        let state = cell.get_or_insert_with(|| State { owner, bars: vec![] });
        while state.bars.len() > wanted.len() {
            if let Some(bar) = state.bars.pop() {
                DestroyWindow(bar.hwnd);
            }
        }
        for (index, (rect, tabs)) in wanted.into_iter().enumerate() {
            if index == state.bars.len() {
                let hwnd = CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    wide(CLASS).as_ptr(),
                    wide("Omarchy Overlay group tabs").as_ptr(),
                    WS_POPUP,
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    null_mut(),
                    null_mut(),
                    GetModuleHandleW(null()),
                    null(),
                );
                if hwnd.is_null() {
                    return;
                }
                crate::desktops::follow(hwnd);
                state.bars.push(Bar { hwnd, tabs: vec![] });
            }
            let bar = &mut state.bars[index];
            let changed = bar.tabs != tabs;
            bar.tabs = tabs;
            SetWindowPos(
                bar.hwnd,
                HWND_TOP,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            if changed {
                InvalidateRect(bar.hwnd, null(), 0);
            }
        }
    });
}
pub(super) unsafe fn stop() {
    BARS.with(|cell| {
        if let Some(state) = cell.borrow_mut().take() {
            for bar in state.bars {
                DestroyWindow(bar.hwnd);
            }
        }
    });
}
unsafe fn paint(bar: &Bar) {
    let p = theme::current();
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(bar.hwnd, &mut ps);
    let mut r: RECT = zeroed();
    GetClientRect(bar.hwnd, &mut r);
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, r.right.max(1), r.bottom.max(1));
    let old_bitmap = SelectObject(dc, bitmap as HGDIOBJ);
    SetDCBrushColor(dc, p.background);
    FillRect(dc, &r, GetStockObject(DC_BRUSH) as HBRUSH);
    let dpi = dpi_of(bar.hwnd);
    let font = fonts::create(crate::font_names::NL_FAMILY, 10, false, dpi as u32);
    let old_font = SelectObject(dc, font as HGDIOBJ);
    SetBkMode(dc, TRANSPARENT as i32);
    let count = bar.tabs.len().max(1) as i32;
    let gap = 2 * dpi / 96;
    for (i, (title, shown)) in bar.tabs.iter().enumerate() {
        let left = r.right * i as i32 / count;
        let right = r.right * (i as i32 + 1) / count - gap;
        let tab = RECT { left, top: 0, right, bottom: r.bottom };
        SetDCBrushColor(dc, if *shown { p.accent } else { p.surface });
        FillRect(dc, &tab, GetStockObject(DC_BRUSH) as HBRUSH);
        SetTextColor(dc, if *shown { p.background } else { p.text });
        let text = wide(title);
        let mut tr = RECT { left: left + 8 * dpi / 96, right: right - 8 * dpi / 96, ..tab };
        DrawTextW(dc, text.as_ptr(), (text.len() - 1) as i32, &mut tr, DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX | DT_CENTER);
    }
    BitBlt(screen, 0, 0, r.right, r.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old_font);
    DeleteObject(font as HGDIOBJ);
    SelectObject(dc, old_bitmap);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(dc);
    EndPaint(bar.hwnd, &ps);
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE as isize;
    }
    if msg == WM_ERASEBKGND {
        return 1;
    }
    let handled = BARS.with(|cell| {
        let cell = cell.try_borrow().ok()?;
        let state = cell.as_ref()?;
        let index = state.bars.iter().position(|b| b.hwnd == hwnd)?;
        let bar = &state.bars[index];
        match msg {
            WM_PAINT => {
                paint(bar);
                Some(0)
            }
            WM_LBUTTONUP => {
                let mut r: RECT = zeroed();
                GetClientRect(hwnd, &mut r);
                let x = (l & 0xffff) as i16 as i32;
                let tab = (x * bar.tabs.len() as i32 / r.right.max(1)) as usize;
                PostMessageW(state.owner, CLICK, index, tab as isize);
                Some(0)
            }
            _ => None,
        }
    });
    handled.unwrap_or_else(|| DefWindowProcW(hwnd, msg, w, l))
}
impl App {
    /// The tile left for a group's shown window once its tab bar has its row; records
    /// the bar's place for `groupbar_poll`. Other windows keep `target`.
    pub(super) unsafe fn groupbar_room(&mut self, hwnd: HWND, target: Rect) -> Rect {
        let key = hwnd as isize;
        let grouped = self
            .groups
            .iter()
            .any(|g| g.members.len() > 1 && g.members[g.shown] == key);
        if !grouped {
            self.groupbar_rects.remove(&key);
            return target;
        }
        let height = HEIGHT * dpi_of(hwnd) / 96;
        if target.h <= 2 * height {
            return target;
        }
        self.groupbar_rects.insert(key, Rect { h: height, ..target });
        Rect { y: target.y + height, h: target.h - height, ..target }
    }
    /// From the main timer: one bar over each group whose shown window is on screen.
    pub(super) unsafe fn groupbar_poll(&mut self) {
        let mut wanted = vec![];
        let mut targets = vec![];
        for group in &self.groups {
            let shown = group.members[group.shown];
            let Some(&rect) = self.groupbar_rects.get(&shown) else {
                continue;
            };
            if group.members.len() < 2
                || !self.active
                || IsWindow(shown as HWND) == 0
                || IsIconic(shown as HWND) != 0
                || self.parked.contains_key(&shown)
            {
                continue;
            }
            let tabs = group
                .members
                .iter()
                .enumerate()
                .map(|(i, &k)| (text_of(k as HWND), i == group.shown))
                .collect();
            wanted.push((rect, tabs));
            targets.push(group.members.clone());
        }
        self.groupbar_rects
            .retain(|k, _| self.groups.iter().any(|g| g.members.len() > 1 && g.members[g.shown] == *k));
        self.groupbar_targets = targets;
        update(self.main, wanted);
    }
    /// A tab was clicked: show that member of the group.
    pub(super) unsafe fn groupbar_click(&mut self, bar: usize, tab: usize) {
        let Some(&key) = self.groupbar_targets.get(bar).and_then(|m| m.get(tab)) else {
            return;
        };
        let Some(g) = self.groups.iter().position(|g| g.members.contains(&key)) else {
            return;
        };
        let current = self.groups[g].shown;
        let len = self.groups[g].members.len() as i32;
        let Some(index) = self.groups[g].members.iter().position(|k| *k == key) else {
            return;
        };
        let step = index as i32 - current as i32;
        if step != 0 {
            self.cycle_group(self.groups[g].members[current] as HWND, step.rem_euclid(len));
        }
    }
}
