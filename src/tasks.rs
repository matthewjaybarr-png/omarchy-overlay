//! Taskbar-style window discovery and activation, restricted to the current native desktop.
use super::*;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
pub(super) struct Identity {
    pub label: String,
    pub icon: HICON,
    /// Where the icon came from; a browser window's own, class, program or generic icon
    /// is rechecked because Chrome and Edge set a web app's ID and icon shortly after it
    /// opens (0.19: the ChatGPT web app kept Chrome's first icon).
    pub source: &'static str,
    browser: bool,
    made: Instant,
    checks: u8,
}
impl Identity {
    // Only browsers: asking a busy app for its icon can hold Omarchy Overlay up to 25 ms per call.
    fn stale(&self) -> bool {
        self.browser
            && matches!(self.source, "window" | "class" | "program" | "generic")
            && self.checks < 6
            && self.made.elapsed() > Duration::from_secs(3)
    }
}
impl Drop for Identity {
    fn drop(&mut self) {
        if !self.icon.is_null() {
            unsafe {
                DestroyIcon(self.icon);
            }
        }
    }
}
thread_local! { static IDENTITIES: RefCell<HashMap<(isize,u32),Rc<Identity>>> = RefCell::new(HashMap::new()); }
type ShortcutIndex = (Instant, HashMap<String, PathBuf>);
// Start-menu shortcuts by AppUserModelID; rebuilt at most once a minute when an ID is missing.
thread_local! { static SHORTCUTS: RefCell<Option<ShortcutIndex>> = const { RefCell::new(None) }; }
fn start_menu_shortcuts(found: &mut HashMap<String, PathBuf>, folder: &Path, depth: usize) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten().take(2000) {
        let path = entry.path();
        if path.is_dir() && depth < 4 {
            start_menu_shortcuts(found, &path, depth + 1);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            if let Some(id) = unsafe { native_status::shortcut_app_id(&path) } {
                found.entry(id.to_lowercase()).or_insert(path);
            }
        }
    }
}
fn shortcut_for(app_id: &str) -> Option<PathBuf> {
    let key = app_id.to_lowercase();
    SHORTCUTS.with(|cell| {
        let mut cell = cell.borrow_mut();
        let fresh = cell.as_ref().is_some_and(|(made, map)| {
            map.contains_key(&key) || made.elapsed() < Duration::from_secs(60)
        });
        if !fresh {
            let mut found = HashMap::new();
            for root in ["APPDATA", "ProgramData"] {
                if let Some(base) = std::env::var_os(root) {
                    let folder =
                        PathBuf::from(base).join("Microsoft\\Windows\\Start Menu\\Programs");
                    start_menu_shortcuts(&mut found, &folder, 0);
                }
            }
            *cell = Some((Instant::now(), found));
        }
        cell.as_ref().and_then(|(_, map)| map.get(&key).cloned())
    })
}
/// True for the AppUserModelID Chrome and Edge give an installed web app's window, such as
/// `Chrome._crx_cadlkienfkclaiaibeoongdcgmdikeeg` (the browser itself has none or another).
fn web_app_id(app_id: &str) -> bool {
    app_id.to_ascii_lowercase().contains("._crx_")
}
unsafe fn shortcut_icon(shortcut: &Path) -> HICON {
    let mut info: SHFILEINFOW = zeroed();
    if SHGetFileInfoW(
        wide(&shortcut.to_string_lossy()).as_ptr(),
        0,
        &mut info,
        size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON | SHGFI_SMALLICON,
    ) != 0
    {
        info.hIcon
    } else {
        null_mut()
    }
}
/// Own copy of a window's icon. Order: for a browser web-app window (0.19), the Start-menu
/// shortcut with its AppUserModelID, then its relaunch icon; then the window's small then
/// big icon, the relaunch icon, the shortcut with the same AppUserModelID, the class icon,
/// the program file's icon, then the generic Windows icon.
unsafe fn window_icon(hwnd: HWND, path: Option<&Path>, app: &(Option<String>, Option<String>)) -> (HICON, &'static str) {
    let (app_id, relaunch) = app;
    if app_id.as_deref().is_some_and(web_app_id) {
        if let Some(shortcut) = app_id.as_deref().and_then(shortcut_for) {
            let icon = shortcut_icon(&shortcut);
            if !icon.is_null() {
                return (icon, "shortcut");
            }
        }
        if let Some(resource) = relaunch {
            let icon = native_status::resource_icon(resource);
            if !icon.is_null() {
                return (icon, "relaunch");
            }
        }
    }
    for kind in [ICON_SMALL2, ICON_SMALL, ICON_BIG] {
        let mut borrowed = 0;
        SendMessageTimeoutW(
            hwnd,
            WM_GETICON,
            kind as usize,
            0,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            25,
            &mut borrowed,
        );
        if borrowed != 0 {
            let icon = CopyImage(borrowed as HANDLE, IMAGE_ICON, 16, 16, 0) as HICON;
            if !icon.is_null() {
                return (icon, "window");
            }
        }
    }
    if let Some(resource) = relaunch {
        let icon = native_status::resource_icon(resource);
        if !icon.is_null() {
            return (icon, "relaunch");
        }
    }
    if let Some(shortcut) = app_id.as_deref().and_then(shortcut_for) {
        let icon = shortcut_icon(&shortcut);
        if !icon.is_null() {
            return (icon, "shortcut");
        }
    }
    for index in [GCLP_HICONSM, GCLP_HICON] {
        let borrowed = GetClassLongPtrW(hwnd, index);
        if borrowed != 0 {
            let icon = CopyImage(borrowed as HANDLE, IMAGE_ICON, 16, 16, 0) as HICON;
            if !icon.is_null() {
                return (icon, "class");
            }
        }
    }
    if let Some(path) = path {
        let mut info: SHFILEINFOW = zeroed();
        if SHGetFileInfoW(
            wide(&path.to_string_lossy()).as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        ) != 0
            && !info.hIcon.is_null()
        {
            return (info.hIcon, "program");
        }
    }
    (CopyIcon(LoadIconW(null_mut(), IDI_APPLICATION)), "generic")
}
unsafe fn file_description(path: &Path) -> Option<String> {
    let path = wide(&path.to_string_lossy());
    let size = GetFileVersionInfoSizeW(path.as_ptr(), null_mut());
    if size == 0 || size > 1024 * 1024 {
        return None;
    }
    let mut bytes = vec![0u8; size as usize];
    if GetFileVersionInfoW(path.as_ptr(), 0, size, bytes.as_mut_ptr().cast()) == 0 {
        return None;
    }
    let mut value = null_mut();
    let mut length = 0;
    let mut translations = vec![(0x0409u16, 0x04b0u16)];
    if VerQueryValueW(
        bytes.as_ptr().cast(),
        wide("\\VarFileInfo\\Translation").as_ptr(),
        &mut value,
        &mut length,
    ) != 0
        && !value.is_null()
    {
        for offset in (0..length.min(256) as usize / 4).map(|i| i * 4) {
            let data = value.cast::<u8>().add(offset);
            translations.insert(
                0,
                (
                    std::ptr::read_unaligned(data.cast::<u16>()),
                    std::ptr::read_unaligned(data.add(2).cast::<u16>()),
                ),
            );
        }
    }
    for (language, codepage) in translations {
        let query = wide(&format!(
            "\\StringFileInfo\\{language:04x}{codepage:04x}\\FileDescription"
        ));
        if VerQueryValueW(
            bytes.as_ptr().cast(),
            query.as_ptr(),
            &mut value,
            &mut length,
        ) != 0
            && !value.is_null()
            && (2..=256).contains(&length)
        {
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(
                value.cast::<u16>(),
                length as usize - 1,
            ))
            .trim()
            .to_owned();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}
unsafe fn identity(hwnd: HWND, title: &str) -> Identity {
    let path = process_path(hwnd);
    let name = path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let known = match name.as_str() {
        "excel.exe" => Some("Excel"),
        "msedge.exe" => Some("Edge"),
        "explorer.exe" => Some("File Explorer"),
        "outlook.exe" | "olk.exe" => Some("Outlook"),
        "winword.exe" => Some("Word"),
        "powerpnt.exe" => Some("PowerPoint"),
        "notepad.exe" => Some("Notepad"),
        "powershell.exe" | "pwsh.exe" => Some("PowerShell"),
        "windowsterminal.exe" => Some("Terminal"),
        "chrome.exe" => Some("Chrome"),
        "firefox.exe" => Some("Firefox"),
        "code.exe" => Some("VS Code"),
        "applicationframehost.exe" => Some(title),
        _ => None,
    };
    let browser = matches!(name.as_str(), "chrome.exe" | "msedge.exe");
    // Only a browser asks for the window's app ID: the call goes through COM.
    let app = if browser {
        native_status::window_app(hwnd)
    } else {
        (None, None)
    };
    // A web app window is named after its Start-menu shortcut ("ChatGPT", not "Chrome").
    let web_app = app
        .0
        .as_deref()
        .filter(|id| web_app_id(id))
        .and_then(shortcut_for)
        .and_then(|s| s.file_stem().map(|n| n.to_string_lossy().into_owned()));
    let label = web_app
        .or_else(|| known.map(str::to_owned))
        .or_else(|| path.as_ref().and_then(|path| file_description(path)))
        .or_else(|| {
            path.as_ref()
                .and_then(|path| path.file_stem())
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| title.to_owned());
    let app = if browser { app } else { native_status::window_app(hwnd) };
    let (icon, source) = window_icon(hwnd, path.as_deref(), &app);
    Identity {
        label,
        icon,
        source,
        browser,
        made: Instant::now(),
        checks: 0,
    }
}
#[derive(Clone)]
pub(super) struct Task {
    pub hwnd: HWND,
    pub pid: u32,
    pub title: String,
    pub identity: Rc<Identity>,
    pub monitor: HMONITOR,
    pub minimized: bool,
}
impl Task {
    pub(super) unsafe fn valid(&self) -> bool {
        let mut pid = 0;
        GetWindowThreadProcessId(self.hwnd, &mut pid);
        let mut cloaked = 0u32;
        IsWindow(self.hwnd) != 0
            && pid == self.pid
            && IsWindowVisible(self.hwnd) != 0
            && DwmGetWindowAttribute(
                self.hwnd,
                DWMWA_CLOAKED as u32,
                &mut cloaked as *mut _ as _,
                size_of::<u32>() as u32,
            ) >= 0
            && cloaked == 0
    }
}
pub(super) unsafe fn collect() -> Vec<Task> {
    unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 || GetWindowTextLengthW(hwnd) == 0 {
            return 1;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == GetCurrentProcessId() {
            return 1;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if style & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) != 0
            || (style & WS_EX_APPWINDOW == 0 && !GetWindow(hwnd, GW_OWNER).is_null())
        {
            return 1;
        }
        if [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
        ]
        .contains(&window_class(hwnd).as_str())
        {
            return 1;
        }
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            &mut cloaked as *mut _ as _,
            size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return 1;
        }
        let title = text_of(hwnd);
        let identity = IDENTITIES.with(|cell| {
            cell.borrow_mut()
                .entry((hwnd as isize, pid))
                .and_modify(|known| {
                    if known.stale() {
                        let mut next = identity(hwnd, &title);
                        next.checks = known.checks + 1;
                        *known = Rc::new(next);
                    }
                })
                .or_insert_with(|| Rc::new(identity(hwnd, &title)))
                .clone()
        });
        (*(data as *mut Vec<Task>)).push(Task {
            identity,
            hwnd,
            pid,
            title,
            monitor: MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
            minimized: IsIconic(hwnd) != 0,
        });
        1
    }
    let mut tasks: Vec<Task> = vec![];
    EnumWindows(Some(collect), &mut tasks as *mut _ as isize);
    IDENTITIES.with(|cell| {
        cell.borrow_mut().retain(|&(hwnd, pid), _| {
            tasks
                .iter()
                .any(|task| task.hwnd as isize == hwnd && task.pid == pid)
        })
    });
    tasks
}
pub(super) unsafe fn activate(task: &Task, minimize_active: bool) -> bool {
    if !task.valid() {
        return false;
    }
    if minimize_active && GetForegroundWindow() == task.hwnd {
        ShowWindowAsync(task.hwnd, SW_MINIMIZE);
        return true;
    }
    if IsIconic(task.hwnd) != 0 {
        ShowWindowAsync(task.hwnd, SW_RESTORE);
    }
    let popup = GetLastActivePopup(task.hwnd);
    let target = if !popup.is_null() && IsWindowVisible(popup) != 0 {
        popup
    } else {
        task.hwnd
    };
    if SetForegroundWindow(target) == 0 {
        let flash = FLASHWINFO {
            cbSize: size_of::<FLASHWINFO>() as u32,
            hwnd: target,
            dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
            uCount: 3,
            dwTimeout: 0,
        };
        FlashWindowEx(&flash);
        return false;
    }
    true
}
pub(super) const CLASS: &str = "OmarchyOverlay.Tasks";
pub(super) struct Panel {
    pub hwnd: HWND,
    pub entries: Vec<Task>,
    pub filtered: Vec<usize>,
    signature: Vec<(isize, u32, String, bool)>,
    pub monitor: Option<HMONITOR>,
    pub dirty: bool,
}
impl Panel {
    pub(super) unsafe fn new(font: HFONT, brush: HBRUSH) -> Self {
        register_class(CLASS, brush);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            wide(CLASS).as_ptr(),
            wide("Omarchy Overlay — open apps").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            840,
            560,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        let dark: i32 = i32::from(theme::current().dark);
        DwmSetWindowAttribute(hwnd, 20, &dark as *const _ as _, size_of::<i32>() as u32);
        let edit = child(
            hwnd,
            "EDIT",
            "",
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            301,
            24,
            24,
            770,
            34,
        );
        SendMessageW(
            edit,
            EM_SETCUEBANNER,
            0,
            wide("Search open windows…").as_ptr() as isize,
        );
        child(
            hwnd,
            "LISTBOX",
            "",
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
            302,
            24,
            76,
            770,
            300,
        );
        child(hwnd, "STATIC", "", 0, 303, 24, 386, 770, 24);
        for (id, label, x, width) in [
            (304, "Switch to window", 24, 190),
            (305, "Minimize", 230, 140),
            (306, "All monitors", 386, 170),
            (307, "Refresh", 572, 120),
        ] {
            child(
                hwnd,
                "BUTTON",
                label,
                BS_OWNERDRAW as u32 | WS_TABSTOP,
                id,
                x,
                426,
                width,
                38,
            );
        }
        child(
            hwnd,
            "STATIC",
            "Enter or double-click to switch · Escape to close · minimized windows stay listed",
            0,
            0,
            24,
            480,
            770,
            24,
        );
        unsafe extern "system" fn set_font(hwnd: HWND, font: LPARAM) -> i32 {
            SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
            1
        }
        EnumChildWindows(hwnd, Some(set_font), font as isize);
        Self {
            hwnd,
            entries: vec![],
            filtered: vec![],
            signature: vec![],
            monitor: None,
            dirty: true,
        }
    }
    pub(super) unsafe fn open(&mut self, monitor: Option<HMONITOR>) {
        self.monitor = monitor;
        self.dirty = true;
        set_text(GetDlgItem(self.hwnd, 301), "");
        crate::desktops::follow(self.hwnd);
        ShowWindow(self.hwnd, SW_SHOWNORMAL);
        self.refresh();
        SetForegroundWindow(self.hwnd);
        SetFocus(GetDlgItem(self.hwnd, 301));
    }
    pub(super) unsafe fn refresh(&mut self) {
        if IsWindowVisible(self.hwnd) == 0 {
            return;
        }
        let mut entries = collect();
        entries.sort_by_key(|task| task.hwnd as isize);
        entries.retain(|task| self.monitor.is_none_or(|monitor| task.monitor == monitor));
        let signature = entries
            .iter()
            .map(|task| {
                (
                    task.hwnd as isize,
                    task.pid,
                    task.title.clone(),
                    task.minimized,
                )
            })
            .collect::<Vec<_>>();
        if self.dirty || signature != self.signature {
            let previous = self.selected().map(|task| (task.hwnd, task.pid));
            self.entries = entries;
            self.signature = signature;
            self.filter_for(previous);
            self.dirty = false;
        }
    }
    pub(super) unsafe fn filter(&mut self) {
        let previous = self.selected().map(|task| (task.hwnd, task.pid));
        self.filter_for(previous);
    }
    unsafe fn filter_for(&mut self, previous: Option<(HWND, u32)>) {
        let list = GetDlgItem(self.hwnd, 302);
        let query = text_of(GetDlgItem(self.hwnd, 301)).to_lowercase();
        self.filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, task)| {
                query.split_whitespace().all(|word| {
                    task.title.to_lowercase().contains(word)
                        || task.identity.label.to_lowercase().contains(word)
                })
            })
            .map(|(i, _)| i)
            .collect();
        SendMessageW(list, WM_SETREDRAW, 0, 0);
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
        for &i in &self.filtered {
            let task = &self.entries[i];
            let label = format!(
                "{}{}",
                if task.minimized {
                    if scrolling_runtime::owned(task.hwnd) {
                        "[Off view]  "
                    } else {
                        "[Minimized]  "
                    }
                } else {
                    ""
                },
                task.title
            );
            SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        if self.filtered.is_empty() {
            SendMessageW(
                list,
                LB_ADDSTRING,
                0,
                wide("No matching open windows").as_ptr() as isize,
            );
        }
        let select = self
            .filtered
            .iter()
            .position(|i| previous == Some((self.entries[*i].hwnd, self.entries[*i].pid)))
            .unwrap_or(0);
        SendMessageW(list, LB_SETCURSEL, select, 0);
        SendMessageW(list, WM_SETREDRAW, 1, 0);
        InvalidateRect(list, null(), 1);
        set_text(
            GetDlgItem(self.hwnd, 303),
            &format!(
                "{} window(s) · {} · current Windows desktop",
                self.filtered.len(),
                if self.monitor.is_some() {
                    "this monitor"
                } else {
                    "all monitors"
                }
            ),
        );
        EnableWindow(
            GetDlgItem(self.hwnd, 304),
            i32::from(!self.filtered.is_empty()),
        );
        EnableWindow(
            GetDlgItem(self.hwnd, 305),
            i32::from(!self.filtered.is_empty()),
        );
    }
    pub(super) unsafe fn command(&mut self, id: usize, notification: usize) {
        match id {
            301 if notification == EN_CHANGE as usize => {
                self.filter();
            }
            302 if notification == LBN_DBLCLK as usize => self.switch(),
            304 => self.switch(),
            305 => {
                if let Some(task) = self.selected() {
                    if task.valid() {
                        ShowWindowAsync(task.hwnd, SW_MINIMIZE);
                        self.dirty = true;
                    }
                }
            }
            306 => {
                self.monitor = None;
                self.dirty = true;
                self.refresh();
            }
            307 => {
                self.dirty = true;
                self.refresh();
            }
            _ => {}
        }
    }
    unsafe fn selected(&self) -> Option<Task> {
        let selected = SendMessageW(GetDlgItem(self.hwnd, 302), LB_GETCURSEL, 0, 0) as usize;
        self.filtered
            .get(selected)
            .and_then(|i| self.entries.get(*i))
            .cloned()
    }
    pub(super) unsafe fn switch(&mut self) {
        if let Some(task) = self.selected() {
            if task.valid() {
                ShowWindow(self.hwnd, SW_HIDE);
                activate(&task, false);
            } else {
                self.dirty = true;
                self.refresh();
            }
        }
    }
}
