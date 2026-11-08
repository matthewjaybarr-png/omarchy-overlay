//! Explorer owns the icons and menus. UI Automation stays on one background COM
//! thread; only the controller changes popup geometry, with a watchdog journal.
use super::*;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::core::Interface;
use windows::Win32::{
    System::{
        Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        Variant::VARIANT,
    },
    UI::Accessibility::*,
};
const FILE: &str = "tray-popup-restore.json";
const OWNER: &str = "OmarchyOverlay.TrayPopupOwner.v1";
const CLASSES: [&str; 2] = [
    "TopLevelWindowForOverflowXamlIsland",
    "NotifyIconOverflowWindow",
];
type Key = (isize, u32);
fn error() -> String {
    "Windows tray is unavailable. Show the Windows taskbar once, then retry Tray. Windows may have no hidden icons or an unsupported tray layout.".into()
}
unsafe fn key() -> Option<Key> {
    let h = explorer_taskbar();
    if h.is_null() || !process_name(h).eq_ignore_ascii_case("explorer.exe") {
        return None;
    }
    let mut pid = 0;
    GetWindowThreadProcessId(h, &mut pid);
    Some((h as isize, pid))
}
unsafe fn popup() -> HWND {
    for class in CLASSES {
        let h = FindWindowW(wide(class).as_ptr(), null());
        if !h.is_null() && process_name(h).eq_ignore_ascii_case("explorer.exe") {
            return h;
        }
    }
    null_mut()
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    hwnd: usize,
    pid: u32,
    marker: usize,
    rect: [i32; 4],
}
impl Original {
    unsafe fn matches(&self) -> bool {
        let h = self.hwnd as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        IsWindow(h) != 0
            && pid == self.pid
            && CLASSES.contains(&window_class(h).as_str())
            && process_name(h).eq_ignore_ascii_case("explorer.exe")
            && GetPropW(h, wide(OWNER).as_ptr()) as usize == self.marker
    }
}
pub(super) unsafe fn recover(directory: &Path) -> Result<(), String> {
    let path = directory.join(FILE);
    if !path.exists() {
        return Ok(());
    }
    let old: Original = serde_json::from_str(&profile::read_text(&path)?)
        .map_err(|e| format!("Invalid tray recovery record: {e}"))?;
    if old.rect.iter().any(|n| n.unsigned_abs() > 1_000_000)
        || old.rect[2] <= old.rect[0]
        || old.rect[3] <= old.rect[1]
    {
        return Err("Invalid tray popup bounds; recovery record retained.".into());
    }
    if old.matches() {
        let h = old.hwnd as HWND;
        if !responsive(h) {
            return Err("Tray popup is unresponsive; recovery record retained.".into());
        }
        if IsWindowVisible(h) != 0 {
            dismiss_native(old.clone())?;
        }
        if !old.matches() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
            return Ok(());
        }
        if SetWindowPos(
            h,
            null_mut(),
            old.rect[0],
            old.rect[1],
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        ) == 0
        {
            return Err("Tray placement restoration is pending.".into());
        }
        RemovePropW(h, wide(OWNER).as_ptr());
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}
// Hiding this XAML window directly leaves Explorer's flyout-open state set,
// pinning its auto-hide taskbar onscreen. Use the original toggle to dismiss it.
unsafe fn dismiss_native(old: Original) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            let _ = tx.send(Err(error()));
            return;
        }
        let result = (|| -> Result<(), String> {
            if !old.matches() || IsWindowVisible(old.hwnd as HWND) == 0 {
                return Ok(());
            }
            let k = key().ok_or_else(error)?;
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|_| error())?;
            if let Ok(options) = automation.cast::<IUIAutomation2>() {
                let _ = options.SetConnectionTimeout(400);
                let _ = options.SetTransactionTimeout(400);
            }
            let pattern = lookup(&automation, k).map_err(|_| error())?;
            if old.matches() && IsWindowVisible(old.hwnd as HWND) != 0 {
                pattern.Invoke().map_err(|e| {
                    format!("Windows tray invocation failed ({:#010X}).", e.code().0)
                })?;
            }
            for _ in 0..20 {
                if !old.matches() || IsWindowVisible(old.hwnd as HWND) == 0 {
                    return Ok(());
                }
                Sleep(25);
            }
            Err("Windows tray dismissal is pending; recovery record retained.".into())
        })();
        let _ = tx.send(result);
        CoUninitialize();
    });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or_else(|_| {
        Err("Windows tray dismissal timed out; recovery record retained.".into())
    })
}
unsafe fn lookup(
    automation: &IUIAutomation,
    k: Key,
) -> windows::core::Result<IUIAutomationInvokePattern> {
    let element = automation.ElementFromHandle(windows::Win32::Foundation::HWND(k.0 as *mut _))?;
    let id = automation
        .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("SystemTrayIcon"))?;
    let found = element.FindAll(TreeScope_Descendants, &id)?;
    let count = found.Length()?;
    if !(0..=64).contains(&count) {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004005u32 as i32,
        )));
    }
    let mut controls = vec![];
    for i in 0..count {
        let e = found.GetElement(i)?;
        controls.push(crate::tray_match::Control {
            id: "SystemTrayIcon".into(),
            class: e.CurrentClassName()?.to_string(),
            name: e.CurrentName()?.to_string(),
        });
    }
    let index = crate::tray_match::select(&controls).map_err(|reason| {
        // Avoid recording network names or other system-control contents.
        let classes = controls
            .iter()
            .map(|c| c.class.as_str())
            .collect::<Vec<_>>();
        log(&format!(
            "Tray lookup: {reason}; SystemTrayIcon classes: {classes:?}"
        ));
        windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005u32 as i32))
    })?;
    log(&format!("Tray lookup selected {}", controls[index].class));
    found
        .GetElement(index as i32)?
        .GetCurrentPatternAs(UIA_InvokePatternId)
}
unsafe fn position_popup(directory: &Path, anchor: RECT) -> Result<(), String> {
    let h = popup();
    if h.is_null() || IsWindowVisible(h) == 0 {
        return Err(error());
    }
    let mut r: RECT = zeroed();
    if GetWindowRect(h, &mut r) == 0 {
        return Err(error());
    }
    let mut pid = 0;
    GetWindowThreadProcessId(h, &mut pid);
    let old = Original {
        hwnd: h as usize,
        pid,
        marker: GetCurrentProcessId() as usize,
        rect: [r.left, r.top, r.right, r.bottom],
    };
    if directory.join(FILE).exists() {
        return Err("An earlier tray popup still needs restoration.".into());
    }
    if !GetPropW(h, wide(OWNER).as_ptr()).is_null()
        || SetPropW(h, wide(OWNER).as_ptr(), old.marker as HANDLE) == 0
    {
        return Err("Cannot take ownership of tray popup placement.".into());
    }
    if let Err(e) = write_json_atomic(&directory.join(FILE), &old) {
        RemovePropW(h, wide(OWNER).as_ptr());
        return Err(e);
    }
    let monitor = MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return Err(error());
    }
    let gap = (GetDpiForWindow(h).max(96) as i32 * 6 / 96).max(1);
    let area = info.rcWork;
    let x = (anchor.right - (r.right - r.left))
        .clamp(area.left, (area.right - (r.right - r.left)).max(area.left));
    let y = (anchor.bottom + gap).clamp(area.top, (area.bottom - (r.bottom - r.top)).max(area.top));
    if SetWindowPos(
        h,
        null_mut(),
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    ) == 0
    {
        return Err("Windows refused tray popup placement.".into());
    }
    Ok(())
}
enum Request {
    Prepare(Key),
    Toggle(Key, bool),
}
enum Reply {
    Prepared(Key, Result<(), String>),
    Toggled(Result<(), String>),
}
#[derive(Clone, Copy)]
enum Busy {
    Prepare,
    Toggle,
}
pub(super) struct Client {
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Reply>,
    attempted: Option<Key>,
    ready: bool,
    busy: Option<(Busy, Instant)>,
    click: Option<(RECT, Instant)>,
    opening: Option<(RECT, Instant)>,
    pub(super) status: String,
    timed_out: bool,
    dismiss_click: Option<Instant>,
    anchor: Option<RECT>,
}
impl Client {
    pub(super) fn new() -> Self {
        let (tx, requests) = mpsc::channel();
        let (replies, rx) = mpsc::channel();
        std::thread::spawn(move || unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return;
            }
            let automation: windows::core::Result<IUIAutomation> =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER);
            if let Ok(automation) = automation {
                if let Ok(options) = automation.cast::<IUIAutomation2>() {
                    let _ = options.SetConnectionTimeout(1000);
                    let _ = options.SetTransactionTimeout(1000);
                }
                let mut cached: Option<(Key, IUIAutomationInvokePattern)> = None;
                while let Ok(request) = requests.recv() {
                    match request {
                        Request::Prepare(k) => {
                            cached = None;
                            let result = lookup(&automation, k);
                            let result = match result {
                                Ok(pattern) if key() == Some(k) => {
                                    cached = Some((k, pattern));
                                    Ok(())
                                }
                                Err(e) => Err(format!("Tray control lookup failed ({:#010X}). The tray layout may differ or Windows may be denying accessibility access. See omarchy-overlay.log for detected control classes.",e.code().0)),
                                _ => Err("Explorer changed while preparing the tray. Retry Tray.".into()),
                            };
                            log(&format!(
                                "Tray preparation: {}",
                                result.as_ref().err().map_or("ready", |e| e.as_str())
                            ));
                            if replies.send(Reply::Prepared(k, result)).is_err() {
                                break;
                            }
                        }
                        Request::Toggle(k, closing) => {
                            let result = match &cached {
                                Some((old, pattern)) if *old == k && key() == Some(k) => {
                                    let h = popup();
                                    if closing && (h.is_null() || IsWindowVisible(h) == 0) {
                                        Ok(())
                                    } else {
                                        pattern.Invoke().map_err(|e| {
                                            format!(
                                                "Windows tray invocation failed ({:#010X}).",
                                                e.code().0
                                            )
                                        })
                                    }
                                }
                                _ => Err(error()),
                            };
                            if replies.send(Reply::Toggled(result)).is_err() {
                                // Controller exited during Invoke. No popup geometry
                                // was changed; dismiss only this Explorer popup.
                                if key() == Some(k) {
                                    let h = popup();
                                    if !h.is_null() && IsWindowVisible(h) != 0 {
                                        if let Some((_, pattern)) = &cached {
                                            let _ = pattern.Invoke();
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            CoUninitialize();
        });
        Self {
            tx,
            rx,
            timed_out: false,
            dismiss_click: None,
            anchor: None,
            attempted: None,
            ready: false,
            busy: None,
            click: None,
            opening: None,
            status: "Preparing Windows tray…".into(),
        }
    }
    pub(super) unsafe fn needs_prepare(&self) -> bool {
        self.busy.is_none() && key() != self.attempted
    }
    pub(super) fn warming(&self) -> bool {
        matches!(self.busy, Some((Busy::Prepare, _))) && !self.timed_out
    }
    pub(super) unsafe fn prepare(&mut self) {
        if self.busy.is_some() {
            return;
        }
        self.timed_out = false;
        self.attempted = key();
        self.ready = false;
        if let Some(k) = self.attempted {
            if self.tx.send(Request::Prepare(k)).is_ok() {
                self.busy = Some((Busy::Prepare, Instant::now()));
                self.status = "Preparing Windows tray…".into();
                return;
            }
        }
        self.status = error();
    }
    pub(super) fn click(&mut self, anchor: RECT) {
        // Mouse-down on this very button may have dismissed Explorer before
        // mouse-up reaches us. Consume that dismissal, not a later genuine click.
        if self
            .dismiss_click
            .take()
            .is_some_and(|t| t.elapsed() < Duration::from_secs(2))
        {
            return;
        }
        // Repeated clicks during an operation coalesce, rather than toggle twice.
        if self.busy.is_some() || self.opening.is_some() || self.click.is_some() {
            return;
        }
        if !self.ready {
            self.attempted = None;
        }
        self.anchor = Some(anchor);
        self.click = Some((anchor, Instant::now()));
    }
    pub(super) unsafe fn poll(&mut self, directory: &Path) -> Option<String> {
        let mut failure = None;
        while let Ok(reply) = self.rx.try_recv() {
            self.busy = None;
            self.timed_out = false;
            match reply {
                Reply::Prepared(k, result) => {
                    self.ready = result.is_ok() && key() == Some(k);
                    self.status = result
                        .err()
                        .unwrap_or_else(|| "Windows tray · original app icons and menus".into());
                }
                Reply::Toggled(result) => {
                    if let Err(e) = result {
                        self.ready = false;
                        self.opening = None;
                        failure = Some(e);
                    }
                }
            }
        }
        if !self.timed_out
            && self
                .busy
                .is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4))
        {
            self.timed_out = true;
            self.ready = false;
            self.status =
                "Windows tray did not respond. Show the Windows taskbar and try again.".into();
            self.click = None;
            self.opening = None;
            // Keep busy until the worker replies: never queue more COM calls.
            if matches!(self.busy, Some((Busy::Toggle, _))) {
                failure = Some(self.status.clone());
            }
        }
        if let Some((anchor, since)) = self.click {
            if since.elapsed() > Duration::from_secs(3) {
                self.click = None;
                failure = Some("Tray action expired while modifier keys remained held.".into());
            } else if [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                .iter()
                .all(|k| GetAsyncKeyState(*k as i32) >= 0)
                && self.busy.is_none()
            {
                self.click = None;
                if self.ready && key() == self.attempted {
                    let closing = directory.join(FILE).exists();
                    if closing && IsWindowVisible(popup()) == 0 {
                        if let Err(e) = recover(directory) {
                            return Some(e);
                        }
                        return None;
                    }
                    if let Some(k) = self.attempted {
                        let _ = self.tx.send(Request::Toggle(k, closing));
                        self.busy = Some((Busy::Toggle, Instant::now()));
                        if !closing {
                            self.opening = Some((anchor, Instant::now()));
                        }
                    }
                } else {
                    failure = Some(self.status.clone());
                }
            }
        }
        if let Some((anchor, since)) = self.opening {
            let h = popup();
            if !h.is_null() && IsWindowVisible(h) != 0 {
                // Wait for Invoke to finish before changing placement.
                if self.busy.is_none() {
                    self.opening = None;
                    if let Err(e) = position_popup(directory, anchor) {
                        let _ = recover(directory);
                        failure = Some(e);
                    }
                }
            } else if self.busy.is_none() && since.elapsed() > Duration::from_secs(3) {
                self.opening = None;
                failure = Some(error());
            }
        }
        if directory.join(FILE).exists() {
            let h = popup();
            if h.is_null() || IsWindowVisible(h) == 0 {
                if let Err(e) = recover(directory) {
                    failure = Some(e);
                } else if GetAsyncKeyState(VK_LBUTTON as i32) < 0 {
                    let mut point: POINT = zeroed();
                    if GetCursorPos(&mut point) != 0
                        && self.anchor.is_some_and(|r| {
                            point.x >= r.left
                                && point.x < r.right
                                && point.y >= r.top
                                && point.y < r.bottom
                        })
                    {
                        self.dismiss_click = Some(Instant::now());
                    }
                }
            }
        }
        if let Some(e) = &failure {
            self.status = e.clone();
        }
        failure
    }
}
