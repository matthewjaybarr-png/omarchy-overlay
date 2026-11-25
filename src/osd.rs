//! The on-screen message, after Omarchy's OSD (shell/plugins/osd): a small card at the
//! top of the screen with an icon, the text and, for volume and brightness, a level bar,
//! in the theme's colors. It never takes focus or clicks.
use super::*;
use std::cell::RefCell;
const CLASS: &str = "OmarchyOverlay.OSD";
/// Nerd Font (Material Design) icons.
pub(super) const INFO: char = '\u{F02FC}';
pub(super) const VOLUME: char = '\u{F057E}';
pub(super) const VOLUME_OFF: char = '\u{F0581}';
pub(super) const BRIGHTNESS: char = '\u{F00DF}';
pub(super) const WORKSPACE: char = '\u{F0379}';
pub(super) const MIC: char = '\u{F036C}';
pub(super) const MIC_OFF: char = '\u{F036D}';
/// Card size at 96 DPI.
const WIDTH: i32 = 340;
const HEIGHT: i32 = 64;
#[derive(Clone, Default)]
struct Card {
    font: String,
    icon: Option<char>,
    text: String,
    level: Option<u8>,
}
thread_local! { static CARD: RefCell<Card> = RefCell::new(Card::default()); }
pub(super) unsafe fn create(_font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay feedback").as_ptr(),
        WS_POPUP,
        0,
        0,
        WIDTH,
        HEIGHT,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    SetLayeredWindowAttributes(hwnd, 0, 245, LWA_ALPHA);
    let corner = 2u32; // DWMWCP_ROUND
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
        &corner as *const _ as _,
        size_of::<u32>() as u32,
    );
    hwnd
}
fn blend(over: u32, under: u32, amount: f32) -> u32 {
    let mix = |shift: u32| {
        let a = ((over >> shift) & 255) as f32;
        let b = ((under >> shift) & 255) as f32;
        ((a * amount + b * (1.0 - amount)).round() as u32) << shift
    };
    mix(0) | mix(8) | mix(16)
}
/// Paints the card; called from the shared window procedure for the OSD class.
pub(super) unsafe fn paint(hwnd: HWND) {
    let card = CARD.with(|c| c.borrow().clone());
    let p = theme::current();
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    let mut r: RECT = zeroed();
    GetClientRect(hwnd, &mut r);
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, r.right.max(1), r.bottom.max(1));
    let old_bitmap = SelectObject(dc, bitmap);
    let fill = |rect: &RECT, color: u32| {
        SetDCBrushColor(dc, color);
        FillRect(dc, rect, GetStockObject(DC_BRUSH) as HBRUSH);
    };
    fill(&r, p.background);
    SetDCBrushColor(dc, p.border);
    FrameRect(dc, &r, GetStockObject(DC_BRUSH) as HBRUSH);
    let dpi = GetDpiForWindow(hwnd).max(96);
    let unit = |v: i32| v * dpi as i32 / 96;
    SetBkMode(dc, TRANSPARENT as i32);
    let mut left = unit(16);
    if let Some(icon) = card.icon.filter(|_| fonts::available()) {
        let font = fonts::create_icon(18, dpi);
        let old = SelectObject(dc, font as HGDIOBJ);
        SetTextColor(dc, p.accent);
        let glyph = wide(&icon.to_string());
        let mut ir = RECT { left, top: 0, right: left + unit(28), bottom: r.bottom };
        DrawTextW(dc, glyph.as_ptr(), (glyph.len() - 1) as i32, &mut ir, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old);
        DeleteObject(font as HGDIOBJ);
        left += unit(40);
    }
    let family = if card.font.is_empty() { crate::font_names::NL_FAMILY } else { card.font.as_str() };
    let font = fonts::create(family, 12, true, dpi);
    let old = SelectObject(dc, font as HGDIOBJ);
    SetTextColor(dc, p.text);
    let text = wide(&card.text);
    let bar_height = unit(6);
    let (text_bottom, bar_top) = if card.level.is_some() {
        (r.bottom / 2 + unit(4), r.bottom / 2 + unit(10))
    } else {
        (r.bottom, 0)
    };
    let mut tr = RECT { left, top: 0, right: r.right - unit(16), bottom: text_bottom };
    DrawTextW(
        dc,
        text.as_ptr(),
        (text.len() - 1) as i32,
        &mut tr,
        DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    SelectObject(dc, old);
    DeleteObject(font as HGDIOBJ);
    if let Some(level) = card.level {
        let track = RECT { left, top: bar_top, right: r.right - unit(16), bottom: bar_top + bar_height };
        fill(&track, blend(p.text, p.background, 0.18));
        let filled = track.left + (track.right - track.left) * i32::from(level.min(100)) / 100;
        fill(&RECT { right: filled, ..track }, p.accent);
    }
    BitBlt(screen, 0, 0, r.right, r.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old_bitmap);
    DeleteObject(bitmap as HGDIOBJ);
    DeleteDC(dc);
    EndPaint(hwnd, &ps);
}
impl App {
    pub(super) unsafe fn show_feedback(&self, text: &str) {
        self.show_card(Some(INFO), text, None);
    }
    /// The text's width on the card, in pixels at `dpi`.
    unsafe fn card_text_width(&self, text: &str, dpi: u32) -> i32 {
        let dc = GetDC(null_mut());
        let family = if self.config.menu_font.is_empty() { crate::font_names::NL_FAMILY } else { self.config.menu_font.as_str() };
        let font = fonts::create(family, 12, true, dpi.max(96));
        let old = SelectObject(dc, font as HGDIOBJ);
        let mut size: SIZE = zeroed();
        let wide_text: Vec<u16> = text.encode_utf16().collect();
        GetTextExtentPoint32W(dc, wide_text.as_ptr(), wide_text.len() as i32, &mut size);
        SelectObject(dc, old);
        DeleteObject(font as HGDIOBJ);
        ReleaseDC(null_mut(), dc);
        size.cx
    }
    /// A notice for the user, as Omarchy's notifications: the themed card for six
    /// seconds, and the log. Text too long for one line of the card, or with the card
    /// turned off, falls back to a Windows message box so nothing is lost.
    pub(super) unsafe fn notice(&self, text: &str) {
        log(text);
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        let (mut dpi, mut unused) = (96u32, 96u32);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut unused);
        let fits = self.config.show_osd
            && !text.contains('\n')
            && GetMonitorInfoW(monitor, &mut info) != 0
            && self.card_text_width(text, dpi) + (16 + 40 + 16) * dpi.max(96) as i32 / 96
                <= (info.rcWork.right - info.rcWork.left) * 8 / 10;
        if fits {
            self.show_card_for(Some(INFO), text, None, 6000);
        } else {
            message(text);
        }
    }
    /// Shows the card: an icon, the text and an optional 0–100 level bar.
    pub(super) unsafe fn show_card(&self, icon: Option<char>, text: &str, level: Option<u8>) {
        self.show_card_for(icon, text, level, 1200);
    }
    /// The card, shown for `ms` milliseconds.
    pub(super) unsafe fn show_card_for(&self, icon: Option<char>, text: &str, level: Option<u8>, ms: u32) {
        if !self.config.show_osd {
            return;
        }
        let target = GetForegroundWindow();
        let monitor = MonitorFromWindow(target, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }
        CARD.with(|c| {
            *c.borrow_mut() = Card {
                font: self.config.menu_font.clone(),
                icon,
                text: text.into(),
                level,
            }
        });
        use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        let (mut dpi, mut unused) = (96u32, 96u32);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut unused);
        let unit = |v: i32| v * dpi.max(96) as i32 / 96;
        // Wide enough for the text, up to most of the screen; longer text ends in "…".
        let text_width = self.card_text_width(text, dpi);
        let most = (info.rcWork.right - info.rcWork.left) * 8 / 10;
        let w = (text_width + unit(16 + 40 + 16)).clamp(unit(WIDTH), most.max(unit(WIDTH)));
        let h = unit(HEIGHT);
        SetWindowPos(
            self.osd,
            HWND_TOPMOST,
            info.rcWork.left + (info.rcWork.right - info.rcWork.left - w) / 2,
            info.rcWork.top + unit(28),
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        InvalidateRect(self.osd, null(), 0);
        SetTimer(self.main, 3, ms, None);
    }
    pub(super) unsafe fn feedback_message(&self, kind: usize) {
        if kind == 0 {
            match native_status::volume() {
                Some((value, true)) => self.show_card(Some(VOLUME_OFF), "Muted", Some(value.min(100))),
                Some((value, false)) => self.show_card(Some(VOLUME), &format!("Volume {value}%"), Some(value.min(100))),
                None => {}
            }
        } else if (2000..=2100).contains(&kind) {
            let value = (kind - 2000) as u8;
            self.show_card(Some(BRIGHTNESS), &format!("Brightness {value}%"), Some(value));
        } else if kind >= 1000 {
            let index = kind - 1000;
            let label = self
                .config
                .bar
                .workspace_labels
                .get(index)
                .map_or("", String::as_str);
            self.show_card(Some(WORKSPACE), &format!("Workspace {}  {label}", index + 1), None);
        }
    }
    pub(super) unsafe fn queue_system_action(&mut self, kind: u32) {
        self.pending_system_action = Some((
            kind,
            std::time::Instant::now() + std::time::Duration::from_secs(3),
        ));
    }
    pub(super) unsafe fn poll_system_action(&mut self) {
        if let Some((kind, deadline)) = self.pending_system_action {
            if std::time::Instant::now() > deadline {
                self.pending_system_action = None;
                log("Native control shortcut expired while modifier keys remained held.");
            } else if crate::desktops::native_control(kind) {
                self.pending_system_action = None;
            }
        }
    }
}
