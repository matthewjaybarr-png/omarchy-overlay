//! The bar's sound panel, after Omarchy's audio panel: the speaker volume on a slider in
//! the theme's colors, mute and microphone toggles, and links to Windows' sound settings
//! and volume mixer. Click or drag on the slider, or use the wheel or the arrow keys; M
//! mutes. Escape or a click anywhere else closes it.
use super::bar_popup::{self, Canvas};
use super::*;

const CLASS: &str = "OmarchyOverlay.AudioPanel";
/// Tells this popup apart from the others in bar_popup.
const ID: usize = 2;
/// Layout at 96 DPI.
const WIDTH: i32 = 330;
const PAD: i32 = 14;
const TITLE: i32 = 32;
const ROW: i32 = 42;
const FOOTER: i32 = 36;
const HEIGHT: i32 = PAD + TITLE + ROW * 2 + FOOTER + 4;
const ICON: i32 = 34;
const PERCENT: i32 = 48;
/// The slider track, left and right, in 96-DPI client units.
const TRACK: (i32, i32) = (PAD + ICON + 8, WIDTH - PAD - PERCENT);

struct Panel {
    hwnd: HWND,
    family: String,
    dpi: u32,
    dragging: bool,
}
thread_local! { static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) }; }

/// Opens the panel by `anchor` (the sound module, in screen pixels), or closes it.
pub(super) unsafe fn toggle(anchor: RECT, family: &str) {
    if close() || bar_popup::just_closed(ID) {
        return;
    }
    let Some((hwnd, dpi)) = bar_popup::open(CLASS, "Omarchy Overlay sound", proc, anchor, WIDTH, HEIGHT) else {
        return;
    };
    PANEL.with(|p| *p.borrow_mut() = Some(Panel { hwnd, family: family.to_string(), dpi, dragging: false }));
    bar_popup::show(hwnd);
}

/// Closes the panel. Returns whether it was open.
pub(super) unsafe fn close() -> bool {
    let Some(hwnd) = PANEL.with(|p| p.borrow_mut().take().map(|p| p.hwnd)) else {
        return false;
    };
    if GetCapture() == hwnd {
        ReleaseCapture();
    }
    DestroyWindow(hwnd);
    true
}

unsafe fn redraw() {
    if let Some(hwnd) = PANEL.with(|p| p.borrow().as_ref().map(|p| p.hwnd)) {
        InvalidateRect(hwnd, null(), 0);
    }
}
/// The volume for a pointer at `x` (96-DPI client units) on the slider.
fn level_at(x: i32) -> u8 {
    (((x - TRACK.0) * 100 + (TRACK.1 - TRACK.0) / 2) / (TRACK.1 - TRACK.0)).clamp(0, 100) as u8
}
unsafe fn nudge(delta: i32) {
    native_status::adjust_volume(delta);
    redraw();
}

unsafe fn paint(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    PANEL.with(|panel| {
        let panel = panel.borrow();
        let Some(panel) = panel.as_ref() else { return };
        let p = theme::current();
        let canvas = Canvas::begin(hwnd, screen, panel.dpi, &panel.family);
        let unit = |v: i32| canvas.unit(v);
        let band = |y: i32, h: i32, x0: i32, x1: i32| RECT { left: unit(x0), top: unit(y), right: unit(x1), bottom: unit(y + h) };
        let dim = bar_popup::blend(p.text, p.background, 0.55);
        let mut y = PAD;
        canvas.text("Sound", band(y, TITLE, PAD, WIDTH - PAD), p.text, true, 11, DT_LEFT);
        y += TITLE;

        // Speaker: mute icon, slider, percent.
        match native_status::volume() {
            Some((level, muted)) => {
                let icon = if muted { osd::VOLUME_OFF } else { osd::VOLUME };
                canvas.icon(icon, band(y, ROW, PAD, PAD + ICON), if muted { dim } else { p.accent }, 16);
                let middle = y + ROW / 2;
                let track = band(middle - 3, 6, TRACK.0, TRACK.1);
                canvas.fill(&track, bar_popup::blend(p.text, p.background, 0.18));
                let filled = track.left + (track.right - track.left) * i32::from(level) / 100;
                canvas.fill(&RECT { right: filled, ..track }, if muted { dim } else { p.accent });
                let knob = unit(6);
                canvas.fill(&RECT { left: filled - knob, right: filled + knob, top: track.top - knob, bottom: track.bottom + knob }, if muted { dim } else { p.text });
                let label = if muted { "Muted".to_string() } else { format!("{level}%") };
                canvas.text(&label, band(y, ROW, WIDTH - PAD - PERCENT + 6, WIDTH - PAD), p.text, false, 10, DT_RIGHT);
            }
            None => {
                canvas.icon(osd::VOLUME_OFF, band(y, ROW, PAD, PAD + ICON), dim, 16);
                canvas.text("No speakers", band(y, ROW, TRACK.0, WIDTH - PAD), dim, false, 10, DT_LEFT);
            }
        }
        y += ROW;

        // Microphone: mute icon and its state.
        let (icon, text, color) = match native_status::mic_muted() {
            Some(true) => (osd::MIC_OFF, "Microphone muted", dim),
            Some(false) => (osd::MIC, "Microphone on", p.accent),
            None => (osd::MIC_OFF, "No microphone", dim),
        };
        canvas.icon(icon, band(y, ROW, PAD, PAD + ICON), color, 16);
        canvas.text(text, band(y, ROW, TRACK.0, WIDTH - PAD), if color == dim { dim } else { p.text }, false, 10, DT_LEFT);
        y += ROW;

        canvas.fill(&band(y + 2, 1, PAD, WIDTH - PAD), bar_popup::blend(p.text, p.background, 0.18));
        canvas.text("Sound settings", band(y + 4, FOOTER - 4, PAD, WIDTH / 2), p.accent, false, 10, DT_CENTER);
        canvas.text("Volume mixer", band(y + 4, FOOTER - 4, WIDTH / 2, WIDTH - PAD), p.accent, false, 10, DT_CENTER);
        canvas.finish();
    });
    EndPaint(hwnd, &ps);
}

/// What a click at (x, y), in 96-DPI client units, does.
#[derive(PartialEq)]
enum Hit {
    Mute,
    Slider,
    Microphone,
    Settings,
    Mixer,
    Nothing,
}
fn hit(x: i32, y: i32) -> Hit {
    let speaker = PAD + TITLE..PAD + TITLE + ROW;
    let mic = speaker.end..speaker.end + ROW;
    if speaker.contains(&y) {
        if x < TRACK.0 - 4 {
            Hit::Mute
        } else if x <= TRACK.1 + 8 {
            Hit::Slider
        } else {
            Hit::Mute
        }
    } else if mic.contains(&y) {
        Hit::Microphone
    } else if y >= mic.end {
        if x < WIDTH / 2 {
            Hit::Settings
        } else {
            Hit::Mixer
        }
    } else {
        Hit::Nothing
    }
}
unsafe fn point(l: LPARAM) -> (i32, i32) {
    let dpi = PANEL.with(|p| p.borrow().as_ref().map(|p| p.dpi)).unwrap_or(96) as i32;
    ((l & 0xffff) as i16 as i32 * 96 / dpi, (l >> 16) as i16 as i32 * 96 / dpi)
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEWHEEL => {
            nudge(if (w >> 16) as i16 > 0 { 2 } else { -2 });
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = point(l);
            if hit(x, y) == Hit::Slider {
                PANEL.with(|p| {
                    if let Some(p) = p.borrow_mut().as_mut() {
                        p.dragging = true;
                    }
                });
                SetCapture(hwnd);
                native_status::set_volume(level_at(x));
                redraw();
            }
            0
        }
        WM_MOUSEMOVE => {
            if PANEL.with(|p| p.borrow().as_ref().is_some_and(|p| p.dragging)) {
                native_status::set_volume(level_at(point(l).0));
                redraw();
            }
            0
        }
        WM_LBUTTONUP => {
            let was_dragging = PANEL.with(|p| {
                p.borrow_mut().as_mut().map(|p| std::mem::replace(&mut p.dragging, false)).unwrap_or(false)
            });
            if was_dragging {
                ReleaseCapture();
                return 0;
            }
            let (x, y) = point(l);
            match hit(x, y) {
                Hit::Mute => {
                    native_status::toggle_mute();
                    redraw();
                }
                Hit::Microphone => {
                    native_status::toggle_mic_mute();
                    redraw();
                }
                Hit::Settings => {
                    close();
                    shell(Path::new("ms-settings:sound"), "");
                }
                Hit::Mixer => {
                    close();
                    shell(Path::new("ms-settings:apps-volume"), "");
                }
                Hit::Slider | Hit::Nothing => {}
            }
            0
        }
        WM_KEYDOWN => {
            match w as u16 {
                VK_ESCAPE => {
                    close();
                }
                VK_LEFT | VK_DOWN => nudge(-5),
                VK_RIGHT | VK_UP => nudge(5),
                k if k == b'M' as u16 => {
                    native_status::toggle_mute();
                    redraw();
                }
                _ => {}
            }
            0
        }
        WM_ACTIVATE if (w & 0xffff) as u32 == WA_INACTIVE => {
            if close() {
                bar_popup::closed_by_click(ID);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_ends_give_zero_and_full_volume() {
        assert_eq!(level_at(TRACK.0 - 20), 0);
        assert_eq!(level_at(TRACK.0), 0);
        assert_eq!(level_at(TRACK.1), 100);
        assert_eq!(level_at(TRACK.1 + 20), 100);
        assert_eq!(level_at((TRACK.0 + TRACK.1) / 2), 50);
    }
}
