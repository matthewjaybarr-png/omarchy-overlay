//! Matching line icons drawn directly with GDI, at the bar's physical DPI.
use super::*;
use crate::bar_model::Module;
#[allow(clippy::too_many_arguments)]
pub(super) unsafe fn draw(
    dc: HDC,
    x: i32,
    y: i32,
    size: i32,
    module: Module,
    action: usize,
    active: bool,
    color: u32,
    status: &str,
) {
    // Without the Nerd Font (a bare exe) Segoe UI would draw empty boxes; keep the drawn icons.
    let glyph = fonts::available().then(|| crate::bar_model::omarchy_glyph(module, active, status)).flatten();
    if let Some(glyph) = glyph {
        let font = CreateFontW(
            -size,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide(&fonts::face(fonts::ICON_FAMILY)).as_ptr(),
        );
        let old_font = SelectObject(dc, font);
        let mut buf = [0u16; 2];
        let text = glyph.encode_utf16(&mut buf);
        let mut r = RECT {
            left: x - size / 2,
            top: y - size / 2,
            right: x + size + size / 2,
            bottom: y + size + size / 2,
        };
        SetTextColor(dc, color);
        SetBkMode(dc, TRANSPARENT as i32);
        DrawTextW(
            dc,
            text.as_ptr(),
            text.len() as i32,
            &mut r,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOCLIP,
        );
        SelectObject(dc, old_font);
        DeleteObject(font);
        return;
    }
    let pen = CreatePen(PS_SOLID, (size / 12).max(1), color);
    let old_pen = SelectObject(dc, pen);
    let old_brush = SelectObject(dc, GetStockObject(HOLLOW_BRUSH));
    let p = |v: i32| v * size / 20;
    let line = |a: i32, b: i32, c: i32, d: i32| {
        MoveToEx(dc, x + p(a), y + p(b), null_mut());
        LineTo(dc, x + p(c), y + p(d));
    };
    let rect = |a: i32, b: i32, c: i32, d: i32| {
        Rectangle(dc, x + p(a), y + p(b), x + p(c), y + p(d));
    };
    match module {
        Module::SystemTray => {
            rect(2, 10, 18, 18);
            line(5, 7, 10, 2);
            line(10, 2, 15, 7);
        }
        Module::Start => {
            rect(1, 2, 9, 9);
            rect(11, 2, 19, 9);
            rect(1, 11, 9, 18);
            rect(11, 11, 19, 18);
        }
        Module::Launcher => {
            Ellipse(dc, x + p(2), y + p(2), x + p(14), y + p(14));
            line(12, 12, 19, 19);
        }
        Module::Menu => {
            for yy in [4, 10, 16] {
                line(2, yy, 18, yy);
            }
        }
        Module::Controls => {
            for xx in [4, 10, 16] {
                line(xx, 2, xx, 18);
            }
            rect(2, 5, 6, 9);
            rect(8, 11, 12, 15);
            rect(14, 4, 18, 8);
        }
        Module::WindowScroll => {
            rect(1, 1, 19, 19);
            line(1, 5, 19, 5);
            if action == 206 {
                line(13, 8, 7, 12);
                line(7, 12, 13, 16);
            } else {
                line(7, 8, 13, 12);
                line(13, 12, 7, 16);
            }
        }
        Module::WorkspaceNav => {
            if action == 98 {
                line(13, 3, 6, 10);
                line(6, 10, 13, 17);
            } else {
                line(7, 3, 14, 10);
                line(14, 10, 7, 17);
            }
        }
        Module::Workspaces => {
            rect(1, 1, 19, 19);
            if action < 100 {
                line(1, 5, 19, 5);
                line(6, 19, 14, 19);
            } else {
                let text = wide(&(action - 99).to_string());
                let mut r = RECT {
                    left: x,
                    top: y,
                    right: x + size,
                    bottom: y + size,
                };
                SetTextColor(dc, color);
                DrawTextW(
                    dc,
                    text.as_ptr(),
                    (text.len() - 1) as i32,
                    &mut r,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
            }
        }
        Module::Tiling => {
            if active {
                rect(1, 2, 9, 18);
                rect(11, 2, 19, 9);
                rect(11, 11, 19, 18);
            } else {
                rect(4, 3, 8, 18);
                rect(12, 3, 16, 18);
            }
        }
        Module::Tasks | Module::AppDrawer => {
            rect(1, 1, 8, 8);
            rect(12, 1, 19, 8);
            rect(1, 12, 8, 19);
            rect(12, 12, 19, 19);
        }
        Module::Title => {
            rect(1, 2, 19, 18);
            line(1, 6, 19, 6);
        }
        Module::Clock => {
            Ellipse(dc, x + p(1), y + p(1), x + p(19), y + p(19));
            line(10, 4, 10, 10);
            line(10, 10, 14, 12);
        }
        Module::Sound => {
            line(2, 7, 6, 7);
            line(6, 7, 11, 3);
            line(11, 3, 11, 17);
            line(11, 17, 6, 13);
            line(6, 13, 2, 13);
            line(2, 13, 2, 7);
            if status.starts_with("MUTE") {
                line(14, 7, 19, 13);
                line(19, 7, 14, 13);
            } else {
                line(14, 6, 17, 10);
                line(17, 10, 14, 14);
            }
        }
        Module::Network => {
            if status.starts_with("WI-FI") {
                Arc(
                    dc,
                    x,
                    y,
                    x + size,
                    y + size,
                    x + p(1),
                    y + p(9),
                    x + p(19),
                    y + p(9),
                );
                line(4, 11, 10, 6);
                line(10, 6, 16, 11);
                Ellipse(dc, x + p(8), y + p(13), x + p(12), y + p(17));
            } else {
                rect(4, 1, 16, 10);
                line(10, 10, 10, 15);
                line(2, 15, 18, 15);
                line(2, 13, 2, 18);
                line(18, 13, 18, 18);
                if status.starts_with("OFFLINE") {
                    line(1, 1, 19, 19);
                }
            }
        }
        Module::Power => {
            rect(1, 4, 17, 16);
            rect(17, 8, 19, 12);
            let value = status.lines().next().unwrap_or("");
            if value == "AC" || value.starts_with('+') {
                line(11, 5, 7, 10);
                line(7, 10, 12, 10);
                line(12, 10, 8, 15);
            } else if let Some(percent) = value
                .strip_suffix('%')
                .and_then(|text| text.parse::<i32>().ok())
            {
                let end = 3 + 12 * percent.clamp(0, 100) / 100;
                if end > 3 {
                    let fill = CreateSolidBrush(color);
                    let r = RECT {
                        left: x + p(3),
                        top: y + p(6),
                        right: x + p(end),
                        bottom: y + p(14),
                    };
                    FillRect(dc, &r, fill);
                    DeleteObject(fill);
                }
            }
        }
        Module::Notifications => {
            Arc(
                dc,
                x + p(3),
                y + p(2),
                x + p(17),
                y + p(17),
                x + p(3),
                y + p(13),
                x + p(17),
                y + p(13),
            );
            line(2, 15, 18, 15);
            line(8, 18, 12, 18);
        }
        Module::QuickSettings => {
            for yy in [4, 10, 16] {
                line(2, yy, 18, yy);
            }
            rect(5, 2, 8, 6);
            rect(12, 8, 15, 12);
            rect(7, 14, 10, 18);
        }
        Module::Display => {
            rect(1, 2, 19, 14);
            line(10, 14, 10, 18);
            line(5, 18, 15, 18);
        }
        Module::PowerMenu => {
            Arc(
                dc,
                x + p(2),
                y + p(2),
                x + p(18),
                y + p(18),
                x + p(6),
                y + p(2),
                x + p(14),
                y + p(2),
            );
            line(10, 1, 10, 10);
        }
        Module::Clipboard => {
            rect(4, 3, 17, 19);
            rect(7, 1, 14, 5);
            line(7, 9, 14, 9);
            line(7, 13, 14, 13);
        }
        Module::Capture => {
            rect(1, 5, 19, 17);
            rect(6, 2, 14, 5);
            Ellipse(dc, x + p(6), y + p(7), x + p(14), y + p(15));
        }
        Module::Divider => {}
    }
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    DeleteObject(pen);
}
