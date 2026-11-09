//! Native app icons and themed rows for the app picker.
use super::*;
use std::rc::Rc;
pub(super) struct Icon(pub HICON);
impl Drop for Icon {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                DestroyIcon(self.0);
            }
        }
    }
}
impl App {
    /// The real icon of an app picker entry, loaded once and shared with the menu.
    pub(super) unsafe fn entry_icon(&mut self, index: usize) -> Rc<Icon> {
        let entry = &self.entries[index];
        self.picker_icons
            .entry(entry.key())
            .or_insert_with(|| {
                if let Some(action) = &entry.action {
                    return Rc::new(Icon(match action {
                        command_menu::Action::Command(_) => CopyIcon(app_icon()),
                        command_menu::Action::Window(task) => CopyIcon(task.identity.icon),
                    }));
                }
                let mut target = if entry.arguments.starts_with("\"shell:")
                    || entry.arguments.starts_with("\"::")
                {
                    entry.arguments.trim_matches('"').to_owned()
                } else {
                    entry.path.to_string_lossy().into_owned()
                };
                if !target.contains(['\\', '/', ':']) {
                    if let Some(root) = std::env::var_os("WINDIR") {
                        let path = PathBuf::from(root).join("System32").join(&target);
                        if path.is_file() {
                            target = path.to_string_lossy().into_owned();
                        }
                    }
                }
                let icon = native_status::shell_icon(&target);
                Rc::new(Icon(if icon.is_null() {
                    CopyIcon(LoadIconW(null_mut(), IDI_APPLICATION))
                } else {
                    icon
                }))
            })
            .clone()
    }
    pub(super) unsafe fn draw_picker(&mut self, item: &DRAWITEMSTRUCT) {
        let palette = theme::current();
        let selected = item.itemState & ODS_SELECTED != 0;
        SetDCBrushColor(
            item.hDC,
            if selected {
                crate::material::selection(palette.border, palette.surface, palette.accent)
            } else {
                palette.surface
            },
        );
        FillRect(item.hDC, &item.rcItem, GetStockObject(DC_BRUSH) as HBRUSH);
        if item.itemID == u32::MAX {
            return;
        }
        let dpi = GetDpiForWindow(self.launcher).max(96) as i32;
        let unit = |x: i32| x * dpi / 96;
        let mut text = "No matching apps".to_owned();
        if let Some(&index) = self.filtered.get(item.itemID as usize) {
            let entry = &self.entries[index];
            text = format!(
                "{}{}",
                if self.picker_history.favorite(&entry.key()) {
                    "★  "
                } else {
                    ""
                },
                entry.label
            );
            let icon = self.entry_icon(index);
            DrawIconEx(
                item.hDC,
                item.rcItem.left + unit(8),
                (item.rcItem.top + item.rcItem.bottom - unit(20)) / 2,
                icon.0,
                unit(20),
                unit(20),
                0,
                null_mut(),
                DI_NORMAL,
            );
        }
        let old = SelectObject(item.hDC, self.font as HGDIOBJ);
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(
            item.hDC,
            if selected {
                palette.accent
            } else {
                palette.text
            },
        );
        let mut rect = item.rcItem;
        rect.left += unit(38);
        rect.right -= unit(10);
        let text = wide(&text);
        DrawTextW(
            item.hDC,
            text.as_ptr(),
            (text.len() - 1) as i32,
            &mut rect,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        if item.itemState & ODS_FOCUS != 0 {
            DrawFocusRect(item.hDC, &item.rcItem);
        }
        SelectObject(item.hDC, old);
    }
}
