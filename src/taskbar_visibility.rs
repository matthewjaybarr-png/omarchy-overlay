//! Local, crash-recoverable hiding of Explorer taskbar windows.
use super::*;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
const PROPERTY: &str = "OmarchyOverlay.TaskbarVisibilityOwner.v1";
const FILE: &str = "taskbar-visibility-restore.json";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    hwnd: usize,
    pid: u32,
    marker: usize,
    visible: bool,
}
impl Original {
    unsafe fn matches(&self) -> bool {
        let h = self.hwnd as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        IsWindow(h) != 0
            && pid == self.pid
            && is_taskbar(h)
            && GetPropW(h, wide(PROPERTY).as_ptr()) as usize == self.marker
    }
    unsafe fn restore(&self) -> bool {
        if !self.matches() {
            return true;
        }
        let h = self.hwnd as HWND;
        if !responsive(h) {
            return false;
        }
        ShowWindowAsync(
            h,
            if self.visible {
                SW_SHOWNOACTIVATE
            } else {
                SW_HIDE
            },
        );
        for _ in 0..20 {
            if (IsWindowVisible(h) != 0) == self.visible {
                RemovePropW(h, wide(PROPERTY).as_ptr());
                return true;
            }
            Sleep(20);
        }
        false
    }
}
unsafe fn is_taskbar(h: HWND) -> bool {
    ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"].contains(&window_class(h).as_str())
        && process_name(h).eq_ignore_ascii_case("explorer.exe")
}
unsafe fn taskbars() -> Vec<HWND> {
    unsafe extern "system" fn collect(h: HWND, p: LPARAM) -> i32 {
        if is_taskbar(h) {
            (*(p as *mut Vec<HWND>)).push(h)
        }
        1
    }
    let mut result = vec![];
    EnumWindows(Some(collect), &mut result as *mut _ as isize);
    result
}
pub(super) struct Engine {
    path: PathBuf,
    originals: Vec<Original>,
    hook: HWINEVENTHOOK,
    pending: bool,
}
impl Engine {
    pub(super) unsafe fn new(directory: &Path) -> Self {
        Self {
            path: directory.join(FILE),
            pending: directory.join(FILE).exists(),
            originals: vec![],
            hook: null_mut(),
        }
    }
    fn save(&self) -> Result<(), String> {
        if self.pending {
            return Err(
                "Taskbar visibility recovery must finish before changing this record.".into(),
            );
        }
        if self.originals.is_empty() {
            if self.path.exists() {
                fs::remove_file(&self.path).map_err(|e| e.to_string())?;
            }
            Ok(())
        } else {
            write_json_atomic(&self.path, &self.originals)
        }
    }
    pub(super) unsafe fn enforce(&mut self) -> Result<(), String> {
        if self.originals.is_empty() && self.path.exists() {
            return Err(
                "Taskbar visibility recovery is pending; Keep hidden was not enabled.".into(),
            );
        }
        self.originals.retain(|o| o.matches());
        for h in taskbars() {
            if self
                .originals
                .iter()
                .any(|o| o.hwnd == h as usize && o.matches())
            {
                if IsWindowVisible(h) != 0 {
                    ShowWindowAsync(h, SW_HIDE);
                }
                continue;
            }
            // Do not take ownership of a taskbar already hidden by another utility.
            if IsWindowVisible(h) == 0 {
                continue;
            }
            if elevated(h) || !responsive(h) {
                return Err("Windows did not allow access to this taskbar.".into());
            }
            let mut pid = 0;
            GetWindowThreadProcessId(h, &mut pid);
            let marker = ((GetCurrentProcessId() as usize).wrapping_mul(16777619)
                ^ std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize)
                | 1;
            if SetPropW(h, wide(PROPERTY).as_ptr(), marker as HANDLE) == 0 {
                return Err(
                    "Windows did not allow a recoverable taskbar visibility change.".into(),
                );
            }
            self.originals.push(Original {
                hwnd: h as usize,
                pid,
                marker,
                visible: true,
            });
            if let Err(e) = self.save() {
                self.originals.pop();
                RemovePropW(h, wide(PROPERTY).as_ptr());
                return Err(e);
            }
            ShowWindowAsync(h, SW_HIDE);
        }
        self.save()?;
        if self.hook.is_null() {
            self.hook = SetWinEventHook(
                EVENT_OBJECT_SHOW,
                EVENT_OBJECT_SHOW,
                null_mut(),
                Some(shown),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
        }
        Ok(())
    }
    pub(super) unsafe fn restore(&mut self) -> Result<(), String> {
        if self.pending {
            return Err(
                "Taskbar visibility recovery is pending; its original record was retained.".into(),
            );
        }
        if !self.hook.is_null() {
            UnhookWinEvent(self.hook);
            self.hook = null_mut();
        }
        self.originals.retain(|o| !o.restore());
        self.save()?;
        if self.originals.is_empty() {
            Ok(())
        } else {
            Err(
                "Taskbar visibility restoration is pending. The recovery record was retained."
                    .into(),
            )
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.is_null() {
                UnhookWinEvent(self.hook);
            }
        }
    }
}
unsafe extern "system" fn shown(
    _: HWINEVENTHOOK,
    _: u32,
    h: HWND,
    object: i32,
    child: i32,
    _: u32,
    _: u32,
) {
    if object == 0 && child == 0 && is_taskbar(h) {
        let owner = FindWindowW(wide(MAIN_CLASS).as_ptr(), null());
        if !owner.is_null() {
            PostMessageW(owner, WM_APP + 41, 0, 0);
        }
    }
}
pub(super) unsafe fn recover(directory: &Path) -> Result<bool, String> {
    let path = directory.join(FILE);
    if !path.exists() {
        return Ok(false);
    }
    let originals: Vec<Original> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| e.to_string())?;
    if originals.len() > 256
        || originals
            .iter()
            .any(|o| o.hwnd == 0 || o.pid == 0 || o.marker == 0)
    {
        return Err("Invalid taskbar visibility recovery data; it was preserved.".into());
    }
    let mut engine = Engine {
        path,
        originals,
        hook: null_mut(),
        pending: false,
    };
    engine.restore()?;
    Ok(true)
}
