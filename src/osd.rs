use super::*;
const CLASS: &str = "OmarchyOverlay.OSD";
pub(super) unsafe fn create(font: HFONT, brush: HBRUSH) -> HWND {
    register_class(CLASS, brush);
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
        wide(CLASS).as_ptr(),
        wide("Omarchy Overlay feedback").as_ptr(),
        WS_POPUP,
        0,
        0,
        340,
        64,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        null(),
    );
    SetLayeredWindowAttributes(hwnd, 0, 240, LWA_ALPHA);
    let label = child(hwnd, "STATIC", "", 1, 901, 16, 20, 308, 28);
    SendMessageW(label, WM_SETFONT, font as usize, 1);
    hwnd
}
impl App {
    pub(super) unsafe fn show_feedback(&self, text: &str) {
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
        set_text(GetDlgItem(self.osd, 901), text);
        SetWindowPos(
            self.osd,
            HWND_TOPMOST,
            info.rcWork.left + (info.rcWork.right - info.rcWork.left - 340) / 2,
            info.rcWork.top + 28,
            340,
            64,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        SetTimer(self.main, 3, 1200, None);
    }
    pub(super) unsafe fn feedback_message(&self, kind: usize) {
        if kind == 0 {
            match native_status::volume() {
                Some((_, true)) => self.show_feedback("Volume muted"),
                Some((value, false)) => self.show_feedback(&format!("Volume {value}%")),
                None => {}
            }
        } else if kind >= 1000 {
            let index = kind - 1000;
            let label = self
                .config
                .bar
                .workspace_labels
                .get(index)
                .map_or("", String::as_str);
            self.show_feedback(&format!("Desktop {}  {label}", index + 1));
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
