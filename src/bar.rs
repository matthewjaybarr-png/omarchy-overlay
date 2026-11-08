//! A normal-user appbar; Explorer stays responsible for the real system tray.
use super::*;
use crate::{
    bar_model::{self, BarConfig, Item, Module, Slot},
    desktops,
};
use windows_sys::Win32::{
    NetworkManagement::{IpHelper::*, Ndis::IfOperStatusUp},
    System::{Power::*, SystemInformation::GetLocalTime},
};

const CLASS: &str = "OmarchyOverlay.TopBar";
const CALLBACK: u32 = WM_APP + 30;
const RELAYOUT: u32 = WM_APP + 31;

struct Panel {
    hwnd: HWND,
    monitor: HMONITOR,
    monitor_rect: RECT,
    rect: RECT,
    font: HFONT,
    dpi: u32,
    positioned: bool,
    tooltip: HWND,
    tip_text: Vec<u16>,
    hovered: bool,
}
struct State {
    owner: HWND,
    directory: PathBuf,
    panels: Vec<Panel>,
    visible: bool,
    hide_taskbar: bool,
    manage_taskbar: bool,
    taskbar_mode: String,
    hidden: taskbar_visibility::Engine,
    tray_popup: Option<tray_popup::Client>,
    tray_was_warming: bool,
    taskbar_error: Option<String>,
    original_taskbar: Option<u32>,
    taskbar_hwnd: HWND,
    taskbar_created: u32,
    brush: HBRUSH,
    class_brush: HBRUSH,
    active: bool,
    title: String,
    network: String,
    sound: String,
    power: String,
    clock: String,
    ticks: u32,
    opacity: u8,
    config: BarConfig,
    desktop: Option<crate::desktops::Snapshot>,
    tasks: Vec<tasks::Task>,
}
thread_local! { static BAR: RefCell<Option<State>> = const { RefCell::new(None) }; }

unsafe fn taskbar_data() -> APPBARDATA {
    APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: explorer_taskbar(),
        ..zeroed()
    }
}
unsafe fn taskbar_state() -> u32 {
    SHAppBarMessage(ABM_GETSTATE, &mut taskbar_data()) as u32 & ABS_AUTOHIDE
}
unsafe fn set_taskbar(state: u32) {
    let mut data = taskbar_data();
    data.lParam = (state & ABS_AUTOHIDE) as isize;
    SHAppBarMessage(ABM_SETSTATE, &mut data);
}
unsafe fn monitors() -> Vec<(HMONITOR, RECT)> {
    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _: HDC,
        rect: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        (*(data as *mut Vec<(HMONITOR, RECT)>)).push((monitor, *rect));
        1
    }
    let mut result = vec![];
    EnumDisplayMonitors(
        null_mut(),
        null(),
        Some(callback),
        &mut result as *mut _ as isize,
    );
    result
}
unsafe fn tip_info(panel: &mut Panel) -> TTTOOLINFOW {
    let mut tool: TTTOOLINFOW = zeroed();
    // The app uses the system common controls (v5); exclude the v6 reserved field.
    tool.cbSize = std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32;
    tool.hwnd = panel.hwnd;
    tool.uId = 1;
    tool.lpszText = panel.tip_text.as_mut_ptr();
    tool
}
unsafe fn hide_tip(panel: &mut Panel) {
    let tool = tip_info(panel);
    SendMessageW(
        panel.tooltip,
        TTM_TRACKACTIVATE,
        0,
        &tool as *const _ as isize,
    );
    panel.hovered = false;
}
fn same_rect(a: RECT, b: RECT) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}

pub(super) unsafe fn start(
    owner: HWND,
    directory: &Path,
    visible: bool,
    hide_taskbar: bool,
    taskbar_mode: &str,
    opacity: u8,
    config: &BarConfig,
) {
    if let Err(error) = recover_taskbar(directory) {
        log(&error);
    }
    let brush = CreateSolidBrush(theme::current().background);
    let class_name = wide(CLASS);
    let class = WNDCLASSW {
        lpfnWndProc: Some(proc),
        hInstance: GetModuleHandleW(null()),
        hCursor: LoadCursorW(null_mut(), IDC_HAND),
        hbrBackground: brush,
        lpszClassName: class_name.as_ptr(),
        ..zeroed()
    };
    RegisterClassW(&class);
    let mut state = State {
        owner,
        directory: directory.into(),
        panels: vec![],
        visible,
        hide_taskbar,
        manage_taskbar: hide_taskbar || taskbar_mode != "legacy",
        taskbar_mode: taskbar_mode.into(),
        hidden: taskbar_visibility::Engine::new(directory),
        tray_popup: None,
        tray_was_warming: false,
        taskbar_error: None,
        original_taskbar: None,
        taskbar_hwnd: null_mut(),
        taskbar_created: RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()),
        brush,
        class_brush: brush,
        active: false,
        title: "Omarchy Overlay".into(),
        network: String::new(),
        sound: "SOUND".into(),
        power: String::new(),
        clock: String::new(),
        ticks: 0,
        opacity,
        config: config.clone(),
        desktop: desktops::poll(),
        tasks: vec![],
    };
    state.rebuild();
    BAR.with(|cell| *cell.borrow_mut() = Some(state));
    poll();
}
pub(super) unsafe fn recover_taskbar(directory: &Path) -> Result<bool, String> {
    let path = directory.join("taskbar-restore.json");
    if !path.exists() {
        return Ok(false);
    }
    let original: u32 = serde_json::from_str(&profile::read_text(&path)?)
        .map_err(|e| format!("Invalid taskbar recovery data: {e}"))?;
    if original & !ABS_AUTOHIDE != 0 {
        return Err("Invalid taskbar recovery state; record preserved.".into());
    }
    set_taskbar(original);
    for _ in 0..10 {
        if taskbar_state() == original {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Sleep(50);
    }
    Err("Taskbar recovery did not complete; record preserved for retry.".into())
}
pub(super) unsafe fn stop() {
    BAR.with(|cell| {
        if let Some(mut state) = cell.borrow_mut().take() {
            state.remove_panels();
            state.restore_taskbar();
            if let Err(e) = tray_popup::recover(&state.directory) {
                log(&e);
            }
            DeleteObject(state.brush);
            if state.brush != state.class_brush {
                DeleteObject(state.class_brush);
            }
        }
    });
}
pub(super) unsafe fn set_active(active: bool) {
    BAR.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.active = active;
            state.redraw();
        }
    });
}
pub(super) unsafe fn toggle_visible() {
    BAR.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.visible = !state.visible;
            state.rebuild();
        }
    });
}
/// Windows' own dark tooltip style when Omarchy Overlay's palette is dark (0.21).
unsafe fn theme_tooltip(tooltip: HWND) {
    if tooltip.is_null() {
        return;
    }
    let name = if theme::current().dark { "DarkMode_Explorer" } else { "Explorer" };
    windows_sys::Win32::UI::Controls::SetWindowTheme(tooltip, wide(name).as_ptr(), null());
}
pub(super) unsafe fn appearance(opacity: u8) {
    BAR.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.opacity = opacity;
            let old = state.brush;
            state.brush = CreateSolidBrush(theme::current().background);
            for panel in &state.panels {
                apply_opacity(panel.hwnd, opacity);
                theme_tooltip(panel.tooltip);
            }
            if old != state.class_brush {
                DeleteObject(old);
            }
            state.redraw();
        }
    });
}
unsafe fn apply_opacity(hwnd: HWND, percent: u8) {
    let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if percent < 100 {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
        SetLayeredWindowAttributes(
            hwnd,
            0,
            ((u16::from(percent) * 255 + 50) / 100) as u8,
            LWA_ALPHA,
        );
    } else {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
    }
}
pub(super) unsafe fn configure(visible: bool, hide_taskbar: bool, taskbar_mode: &str, opacity: u8) {
    BAR.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.visible = visible;
            state.hide_taskbar = hide_taskbar;
            state.manage_taskbar = hide_taskbar || taskbar_mode != "legacy";
            state.taskbar_mode = taskbar_mode.into();
            state.opacity = opacity;
            state.rebuild();
        }
    });
}
pub(super) unsafe fn settings(config: &BarConfig) {
    BAR.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.config = config.clone();
            state.rebuild();
        }
    });
}
pub(super) unsafe fn set_taskbar_mode(mode: &str) -> Result<(), String> {
    BAR.with(|cell| {
        let mut state = cell.borrow_mut();
        let state = state.as_mut().ok_or("The bar is not available.")?;
        let old = state.taskbar_mode.clone();
        let old_manage = state.manage_taskbar;
        state.taskbar_mode = mode.into();
        state.manage_taskbar = true;
        state.taskbar_error = None;
        state.apply_taskbar();
        if let Some(error) = state.taskbar_error.take() {
            state.taskbar_mode = old;
            state.manage_taskbar = old_manage;
            state.apply_taskbar();
            return Err(error);
        }
        Ok(())
    })
}
pub(super) fn tray_status() -> String {
    BAR.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|s| s.tray_popup.as_ref())
            .map_or_else(
                || "Windows tray is unavailable.".into(),
                |p| p.status.clone(),
            )
    })
}
pub(super) unsafe fn enforce_hidden() {
    BAR.with(|cell| {
        if let Ok(mut state) = cell.try_borrow_mut() {
            if let Some(s) = state.as_mut() {
                if s.visible && s.taskbar_mode == "hidden" && !s.tray_warming() {
                    if let Err(e) = s.hidden.enforce() {
                        log(&e)
                    }
                }
            }
        }
    });
}
pub(super) unsafe fn poll() {
    let desktop = desktops::poll();
    BAR.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            return;
        };
        let Some(state) = state.as_mut() else {
            return;
        };
        state.poll_tray();
        state.ticks = state.ticks.wrapping_add(1);
        if state.visible && state.ticks % 8 == 1 {
            let monitors = monitors();
            if monitors.len() != state.panels.len()
                || monitors.iter().any(|(m, rect)| {
                    !state
                        .panels
                        .iter()
                        .any(|p| p.monitor == *m && same_rect(p.monitor_rect, *rect))
                })
            {
                state.rebuild();
            }
            let taskbar = explorer_taskbar();
            if !taskbar.is_null() && taskbar != state.taskbar_hwnd {
                state.taskbar_hwnd = taskbar;
                state.rebuild();
            }
            state.network = network_label();
            state.power = power_label();
        }
        if state.visible && state.taskbar_mode == "hidden" && !state.tray_warming() {
            if let Err(e) = state.hidden.enforce() {
                log(&e);
            }
        }
        if state.desktop != desktop {
            if let Some(s) = &desktop {
                PostMessageW(state.owner, OSD_MESSAGE, 1000 + s.current, 0);
            }
            state.desktop = desktop.clone();
            for panel in &state.panels {
                desktops::follow(panel.hwnd);
            }
        }
        let fresh = tasks::collect();
        state.tasks.retain(|old| {
            fresh
                .iter()
                .any(|task| task.hwnd == old.hwnd && task.pid == old.pid)
        });
        for task in &mut state.tasks {
            if let Some(new) = fresh
                .iter()
                .find(|new| new.hwnd == task.hwnd && new.pid == task.pid)
            {
                *task = new.clone();
            }
        }
        for task in fresh {
            if !state
                .tasks
                .iter()
                .any(|old| old.hwnd == task.hwnd && old.pid == task.pid)
            {
                state.tasks.push(task);
            }
        }
        if state.visible && state.ticks % 2 == 1 {
            state.sound = match native_status::volume() {
                Some((_, true)) => "MUTE".into(),
                Some((percent, false)) => format!("VOL {percent}%"),
                None => "SOUND".into(),
            };
        }
        let foreground = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(foreground, &mut pid);
        if pid != GetCurrentProcessId() {
            let title = text_of(foreground);
            if !title.is_empty() {
                state.title = title;
            }
        }
        let mut time: SYSTEMTIME = zeroed();
        GetLocalTime(&mut time);
        state.clock = bar_model::clock_label(
            &state.config.clock_format,
            [time.wYear, time.wMonth, time.wDay],
            time.wDayOfWeek,
            time.wHour,
            time.wMinute,
        );
        state.refresh_tooltips();
        state.redraw();
    });
}

impl State {
    unsafe fn refresh_tooltips(&mut self) {
        let mut cursor: POINT = zeroed();
        GetCursorPos(&mut cursor);
        let mut fresh = vec![];
        for (index, panel) in self
            .panels
            .iter()
            .enumerate()
            .filter(|(_, panel)| panel.hovered)
        {
            let x = (cursor.x - panel.rect.left) * 96 / panel.dpi as i32;
            let width = (panel.rect.right - panel.rect.left) * 96 / panel.dpi as i32;
            let tip = self
                .slots(width, panel.monitor)
                .into_iter()
                .find(|slot| x >= slot.left && x < slot.right)
                .map_or(String::new(), |slot| slot.item.tooltip);
            fresh.push((index, wide(&tip)));
        }
        for (index, text) in fresh {
            let panel = &mut self.panels[index];
            if panel.tip_text != text {
                panel.tip_text = text;
                let tool = tip_info(panel);
                SendMessageW(
                    panel.tooltip,
                    TTM_UPDATETIPTEXTW,
                    0,
                    &tool as *const _ as isize,
                );
                SendMessageW(panel.tooltip, TTM_UPDATE, 0, 0);
            }
        }
    }
    unsafe fn items(&self, modules: &[Module], monitor: HMONITOR) -> Vec<Item> {
        let mut items = vec![];
        let item = |module, label: String, action, active| {
            let mode = self.config.display(module);
            let icon = mode.icon() && module != Module::Divider;
            let tooltip = match module {
                Module::Start => "Windows Start menu".into(),
                Module::Launcher => "Search installed apps\nOpen the Omarchy Overlay app picker".into(),
                Module::Menu | Module::Controls => {
                    "Omarchy Overlay menu\nTiling, apps, appearance, and Windows controls".into()
                }
                Module::SystemTray => self
                    .tray_popup
                    .as_ref()
                    .map_or_else(|| module.description().into(), |p| p.status.clone()),
                Module::WindowScroll => module.description().into(),
                Module::WorkspaceNav => {
                    format!("{label} Windows desktop\nWindows switches all monitors together")
                }
                Module::Workspaces => {
                    format!("Windows desktop: {label}\nClick to switch all monitors")
                }
                Module::Tiling => format!(
                    "Tiling: {}\nClick to {}",
                    if active { "enabled" } else { "paused" },
                    if active {
                        "pause and restore windows"
                    } else {
                        "enable tiling"
                    }
                ),
                Module::Tasks | Module::AppDrawer => {
                    format!("{label}\nOpen windows on this monitor")
                }
                Module::Title => label.clone(),
                Module::Clock => format!("{label}\nOpen Windows notifications and calendar"),
                Module::Sound => {
                    format!("{label}\nClick: Quick Settings · Scroll: volume · Middle-click: mute")
                }
                Module::Network => {
                    format!("{label}\nNetwork link status · Click: Windows Quick Settings")
                }
                Module::Power => format!(
                    "{}\nClick: Windows power settings",
                    if let Some(level) = label.strip_prefix('+') {
                        format!("Battery: {level} · plugged in")
                    } else if label.ends_with('%') {
                        format!("Battery: {label}")
                    } else {
                        label.clone()
                    }
                ),
                Module::Divider => String::new(),
                Module::Notifications
                | Module::QuickSettings
                | Module::Display
                | Module::PowerMenu
                | Module::Clipboard
                | Module::Capture => module.description().into(),
            };
            let label = if mode.text() || module == Module::Divider {
                label
            } else {
                String::new()
            };
            let width = if module == Module::Divider {
                9
            } else if label.is_empty() {
                28
            } else if module == Module::Title {
                200
            } else {
                let text = ((label.chars().count() as i32 * self.config.font_size * 3 + 4) / 5
                    + 18)
                    .max(28);
                (if module == Module::Workspaces {
                    text.min(80)
                } else {
                    text
                }) + if icon { 22 } else { 0 }
            };
            Item {
                module,
                label,
                width,
                action,
                active,
                icon,
                tooltip,
            }
        };
        for &module in modules {
            use Module::*;
            if module == Tasks {
                let entries = self
                    .tasks
                    .iter()
                    .enumerate()
                    .filter(|(_, task)| task.monitor == monitor)
                    .collect::<Vec<_>>();

                for (index, task) in entries.into_iter().take(self.config.task_buttons) {
                    let label = task.identity.label.chars().take(20).collect::<String>();
                    let mut button = item(
                        module,
                        if task.minimized {
                            format!("_ {label}")
                        } else {
                            label
                        },
                        2000 + index,
                        GetForegroundWindow() == task.hwnd,
                    );
                    button.width = if button.label.is_empty() {
                        28
                    } else {
                        button.width.clamp(70, 170)
                    };
                    button.tooltip = format!(
                        "{}\n{}\nClick to activate / minimize · Minimized: {}",
                        task.identity.label,
                        task.title,
                        if task.minimized { "yes" } else { "no" }
                    );
                    items.push(button);
                }
                continue;
            }
            if module == WindowScroll {
                items.push(item(module, "Previous window".into(), 206, false));
                items.push(item(module, "Next window".into(), 207, false));
                continue;
            }
            if module == WorkspaceNav {
                items.push(item(module, "Previous".into(), 98, false));
                items.push(item(module, "Next".into(), 99, false));
                continue;
            }
            if module == Workspaces {
                if let Some(s) = &self.desktop {
                    let page = self.config.workspace_labels.len().min(4);
                    let start = s.current / page * page;
                    for i in start..(start + page).min(s.ids.len()) {
                        let label = self
                            .config
                            .workspace_labels
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| (i + 1).to_string());
                        items.push(item(module, label, 100 + i, i == s.current));
                    }
                } else {
                    items.push(item(module, "DESKTOPS".into(), 90, false));
                }
                continue;
            }
            let (label, action, active) = match module {
                SystemTray => ("TRAY".into(), 208, false),
                Start => ("START".into(), 96, false),
                Launcher => ("APPS".into(), 0, false),
                AppDrawer => (
                    format!(
                        "WINDOWS {}",
                        self.tasks.iter().filter(|t| t.monitor == monitor).count()
                    ),
                    10,
                    false,
                ),
                Menu => ("MENU".into(), 2, true),
                Tiling => (
                    if self.active { "TILE" } else { "PAUSE" }.into(),
                    1,
                    self.active,
                ),
                Title => (self.title.clone(), usize::MAX, false),
                Clock => (self.clock.clone(), 91, false),
                Sound => (self.sound.clone(), 92, false),
                Network => (self.network.clone(), 93, false),
                Power => (self.power.clone(), 94, false),
                Controls => ("≡".into(), 2, false),
                Divider => (String::new(), usize::MAX, false),
                Notifications => ("NOTIFY".into(), 200, false),
                QuickSettings => ("SYSTEM".into(), 201, false),
                Display => ("DISPLAY".into(), 202, false),
                PowerMenu => ("POWER".into(), 203, false),
                Clipboard => ("CLIP".into(), 204, false),
                Capture => ("CAPTURE".into(), 205, false),
                Workspaces | Tasks | WorkspaceNav | WindowScroll => unreachable!(),
            };
            if !label.is_empty() || module == Divider {
                items.push(item(module, label, action, active));
            }
        }
        items
    }
    unsafe fn slots(&self, width: i32, monitor: HMONITOR) -> Vec<Slot> {
        bar_model::arrange(
            width,
            self.config.spacing,
            [
                self.items(&self.config.left, monitor),
                self.items(&self.config.center, monitor),
                self.items(&self.config.right, monitor),
            ],
        )
    }

    unsafe fn remove_panels(&mut self) {
        for panel in self.panels.drain(..) {
            let mut data = APPBARDATA {
                cbSize: size_of::<APPBARDATA>() as u32,
                hWnd: panel.hwnd,
                ..zeroed()
            };
            SHAppBarMessage(ABM_REMOVE, &mut data);
            DestroyWindow(panel.tooltip);
            DestroyWindow(panel.hwnd);
            DeleteObject(panel.font);
        }
    }
    unsafe fn restore_taskbar(&mut self) {
        if let Err(e) = self.hidden.restore() {
            log(&e);
        }
        if let Some(original) = self.original_taskbar.take() {
            set_taskbar(original);
            if taskbar_state() == original {
                let _ = fs::remove_file(self.directory.join("taskbar-restore.json"));
            } else {
                log("Windows taskbar restoration is pending; recovery file kept for next launch.");
            }
        }
    }
    fn tray_warming(&self) -> bool {
        self.tray_popup.as_ref().is_some_and(|p| p.warming())
    }
    unsafe fn poll_tray(&mut self) {
        let enabled = [&self.config.left, &self.config.center, &self.config.right]
            .iter()
            .any(|s| s.contains(&Module::SystemTray));
        if enabled && self.tray_popup.is_none() {
            self.tray_popup = Some(tray_popup::Client::new());
        }
        if !enabled {
            if self.tray_popup.take().is_some() {
                let _ = self.hidden.restore();
                if let Err(e) = tray_popup::recover(&self.directory) {
                    log(&e);
                }
                self.apply_taskbar();
            }
            return;
        }
        let was_warming = self.tray_was_warming;
        if let Some(client) = self.tray_popup.as_mut() {
            if client.needs_prepare() {
                // Release only our hidden-taskbar ownership while acquiring the
                // accessibility cache. Normal taskbar restoration stays journaled.
                if let Err(e) = self.hidden.restore() {
                    log(&e);
                    return;
                }
                client.prepare();
            }
            if let Some(error) = client.poll(&self.directory) {
                log(&error);
                PostMessageW(self.owner, WM_APP + 59, 0, 0);
            }
        }
        self.tray_was_warming = self.tray_warming();
        if was_warming && !self.tray_warming() {
            self.apply_taskbar();
        }
    }
    unsafe fn poll_tray_prepare(&mut self) {
        let enabled = [&self.config.left, &self.config.center, &self.config.right]
            .iter()
            .any(|s| s.contains(&Module::SystemTray));
        if enabled && self.tray_popup.is_none() {
            self.tray_popup = Some(tray_popup::Client::new());
        }
        if let Some(client) = self.tray_popup.as_mut() {
            if enabled && client.needs_prepare() && self.hidden.restore().is_ok() {
                client.prepare();
                self.tray_was_warming = true;
            }
        }
    }
    unsafe fn apply_taskbar(&mut self) {
        self.poll_tray_prepare();
        if self.tray_warming() {
            return;
        }

        if !self.visible || !self.manage_taskbar || self.panels.is_empty() {
            self.restore_taskbar();
            return;
        }
        if self.original_taskbar.is_none() {
            let journal = self.directory.join("taskbar-restore.json");
            if journal.exists() {
                log("Taskbar recovery is still pending; leaving auto-hide unchanged.");
                return;
            }
            let original = taskbar_state();
            if write_json_atomic(&journal, &original).is_err() {
                log("Could not save taskbar preference; leaving auto-hide unchanged.");
                return;
            }
            self.original_taskbar = Some(original);
        }
        let auto = match self.taskbar_mode.as_str() {
            "visible" => false,
            "hidden" | "auto-hide" => true,
            _ => self.hide_taskbar,
        };
        set_taskbar(if auto { ABS_AUTOHIDE } else { 0 });
        if self.taskbar_mode == "hidden" {
            if let Err(e) = self.hidden.enforce() {
                self.taskbar_error = Some(e.clone());
                log(&e);
            }
        } else if let Err(e) = self.hidden.restore() {
            self.taskbar_error = Some(e.clone());
            log(&e);
        }
        self.taskbar_hwnd = explorer_taskbar();
    }
    unsafe fn rebuild(&mut self) {
        self.remove_panels();
        if !self.visible {
            self.restore_taskbar();
            return;
        }
        for (monitor, rect) in monitors() {
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                wide(CLASS).as_ptr(),
                wide("Omarchy Overlay top bar").as_ptr(),
                WS_POPUP,
                rect.left,
                rect.top,
                rect.right - rect.left,
                34,
                null_mut(),
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            );
            if hwnd.is_null() {
                log("Could not create a top bar window.");
                continue;
            }
            let mut data = APPBARDATA {
                cbSize: size_of::<APPBARDATA>() as u32,
                hWnd: hwnd,
                uCallbackMessage: CALLBACK,
                ..zeroed()
            };
            if SHAppBarMessage(ABM_NEW, &mut data) == 0 {
                log("Windows refused the top bar reservation.");
                DestroyWindow(hwnd);
                continue;
            }
            let dpi = GetDpiForWindow(hwnd).max(96);
            let font = CreateFontW(
                -(self.config.font_size * dpi as i32 / 96),
                0,
                0,
                0,
                FW_MEDIUM as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32,
                FIXED_PITCH as u32,
                wide(&self.config.font_family).as_ptr(),
            );
            apply_opacity(hwnd, self.opacity);
            desktops::follow(hwnd);
            let tooltip = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                TOOLTIPS_CLASSW,
                null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            );
            let mut tip_text = wide("");
            let mut tool: TTTOOLINFOW = zeroed();
            theme_tooltip(tooltip);
            // The v1 size is accepted by every common controls version.
            tool.cbSize = std::mem::offset_of!(TTTOOLINFOW, lpReserved) as u32;
            tool.uFlags = TTF_TRACK | TTF_ABSOLUTE;
            tool.hwnd = hwnd;
            tool.uId = 1;
            tool.rect = RECT {
                left: 0,
                top: 0,
                right: rect.right - rect.left,
                bottom: self.config.height * dpi as i32 / 96,
            };
            tool.lpszText = tip_text.as_mut_ptr();
            if SendMessageW(tooltip, TTM_ADDTOOLW, 0, &tool as *const _ as isize) == 0 {
                log("Could not register the bar tooltip.");
            }
            SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, 0, (480 * dpi / 96) as isize);
            SendMessageW(tooltip, TTM_SETDELAYTIME, TTDT_INITIAL as usize, 450);
            self.panels.push(Panel {
                hwnd,
                monitor,
                monitor_rect: rect,
                rect: zeroed(),
                font,
                dpi,
                positioned: false,
                tooltip,
                tip_text,
                hovered: false,
            });
        }
        self.layout();
        self.apply_taskbar();
    }
    unsafe fn layout(&mut self) {
        for panel in &mut self.panels {
            let height = self.config.height * panel.dpi as i32 / 96;
            let mut data = APPBARDATA {
                cbSize: size_of::<APPBARDATA>() as u32,
                hWnd: panel.hwnd,
                uEdge: ABE_TOP,
                rc: panel.monitor_rect,
                ..zeroed()
            };
            data.rc.bottom = data.rc.top + height;
            SHAppBarMessage(ABM_QUERYPOS, &mut data);
            data.rc.bottom = data.rc.top + height;
            if !panel.positioned || !same_rect(panel.rect, data.rc) {
                SHAppBarMessage(ABM_SETPOS, &mut data);
                panel.rect = data.rc;
                panel.positioned = true;
                SetWindowPos(
                    panel.hwnd,
                    HWND_TOPMOST,
                    data.rc.left,
                    data.rc.top,
                    data.rc.right - data.rc.left,
                    data.rc.bottom - data.rc.top,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }
    unsafe fn redraw(&self) {
        for panel in &self.panels {
            InvalidateRect(panel.hwnd, null(), 0);
        }
    }
    unsafe fn menu(&self, hwnd: HWND) {
        let menu = CreatePopupMenu();
        let add = |target: HMENU, id: usize, label: &str| {
            AppendMenuW(target, MF_STRING, id, wide(label).as_ptr());
        };
        add(menu, 3, "Open dashboard");
        add(menu, 1, "Search installed apps");
        add(menu, 11, "Open windows");
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        add(
            menu,
            2,
            if self.active {
                "Pause tiling and restore windows"
            } else {
                "Enable tiling"
            },
        );
        add(menu, 49, "Rebalance tiles");
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        add(menu, 64, "Exit before removing the stick");
        add(menu, 6, "Restore desktop and exit");
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            null(),
        );
        DestroyMenu(menu);
        if selected > 0 {
            PostMessageW(self.owner, BAR_ACTION, selected as usize - 1, 0);
        }
        PostMessageW(hwnd, WM_NULL, 0, 0);
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // Shell messages can arrive synchronously while registering another panel.
    if msg == CALLBACK && w as u32 == ABN_POSCHANGED {
        PostMessageW(hwnd, RELAYOUT, 0, 0);
        return 0;
    }
    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE as isize;
    }
    let handled = BAR.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            return None;
        };
        let state = state.as_mut()?;
        if [WM_LBUTTONUP, WM_RBUTTONUP, WM_MBUTTONUP, WM_MOUSEWHEEL].contains(&msg) {
            if let Some(panel) = state.panels.iter_mut().find(|panel| panel.hwnd == hwnd) {
                hide_tip(panel);
            }
        }
        if msg == state.taskbar_created {
            state.rebuild();
            return Some(0);
        }
        match msg {
            WM_PAINT => {
                paint(state, hwnd);
                Some(0)
            }
            WM_ERASEBKGND => Some(1),
            RELAYOUT => {
                state.layout();
                Some(0)
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE => {
                PostMessageW(hwnd, RELAYOUT, 0, 0);
                Some(0)
            }
            WM_ACTIVATE | WM_WINDOWPOSCHANGED => {
                let mut data = APPBARDATA {
                    cbSize: size_of::<APPBARDATA>() as u32,
                    hWnd: hwnd,
                    ..zeroed()
                };
                SHAppBarMessage(
                    if msg == WM_ACTIVATE {
                        ABM_ACTIVATE
                    } else {
                        ABM_WINDOWPOSCHANGED
                    },
                    &mut data,
                );
                None
            }
            CALLBACK if w as u32 == ABN_FULLSCREENAPP => {
                SetWindowPos(
                    hwnd,
                    if l != 0 { HWND_BOTTOM } else { HWND_TOPMOST },
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                Some(0)
            }
            WM_MOUSEMOVE => {
                let panel = state.panels.iter().find(|panel| panel.hwnd == hwnd)?;
                let x = (l as i16 as i32) * 96 / panel.dpi as i32;
                let width = (panel.rect.right - panel.rect.left) * 96 / panel.dpi as i32;
                let tip = state
                    .slots(width, panel.monitor)
                    .into_iter()
                    .find(|slot| x >= slot.left && x < slot.right)
                    .map_or(String::new(), |slot| slot.item.tooltip);
                let panel = state.panels.iter_mut().find(|panel| panel.hwnd == hwnd)?;
                let text = wide(&tip);
                if !panel.hovered || text != panel.tip_text {
                    panel.hovered = true;
                    let mut tool = tip_info(panel);
                    SendMessageW(
                        panel.tooltip,
                        TTM_TRACKACTIVATE,
                        0,
                        &tool as *const _ as isize,
                    );
                    panel.tip_text = text;
                    tool.lpszText = panel.tip_text.as_mut_ptr();
                    SendMessageW(
                        panel.tooltip,
                        TTM_UPDATETIPTEXTW,
                        0,
                        &tool as *const _ as isize,
                    );
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_HOVER | TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 450,
                    };
                    TrackMouseEvent(&mut track);
                }
                Some(0)
            }
            WM_MOUSEHOVER => {
                let panel = state.panels.iter_mut().find(|panel| panel.hwnd == hwnd)?;
                if panel.hovered && panel.tip_text.len() > 1 {
                    let mut point: POINT = zeroed();
                    GetCursorPos(&mut point);
                    let x = (point.x + 12)
                        .min(panel.monitor_rect.right - 480 * panel.dpi as i32 / 96)
                        .max(panel.monitor_rect.left);
                    let y = point.y + 24;
                    let packed = ((x as u16 as u32) | ((y as u16 as u32) << 16)) as isize;
                    SendMessageW(panel.tooltip, TTM_TRACKPOSITION, 0, packed);
                    let tool = tip_info(panel);
                    SendMessageW(
                        panel.tooltip,
                        TTM_TRACKACTIVATE,
                        1,
                        &tool as *const _ as isize,
                    );
                }
                Some(0)
            }
            WM_MOUSELEAVE => {
                if let Some(panel) = state.panels.iter_mut().find(|panel| panel.hwnd == hwnd) {
                    hide_tip(panel);
                }
                Some(0)
            }
            WM_RBUTTONUP => {
                state.menu(hwnd);
                Some(0)
            }
            WM_MOUSEWHEEL | WM_MBUTTONUP => {
                let panel = state.panels.iter().find(|panel| panel.hwnd == hwnd)?;
                let mut point = POINT {
                    x: l as i16 as i32,
                    y: (l >> 16) as i16 as i32,
                };
                if msg == WM_MOUSEWHEEL {
                    ScreenToClient(hwnd, &mut point);
                }
                let x = point.x * 96 / panel.dpi as i32;
                let width = (panel.rect.right - panel.rect.left) * 96 / panel.dpi as i32;
                if msg == WM_MOUSEWHEEL
                    && state.slots(width, panel.monitor).into_iter().any(|slot| {
                        [Module::Tasks, Module::AppDrawer, Module::WindowScroll]
                            .contains(&slot.item.module)
                            && x >= slot.left
                            && x < slot.right
                    })
                {
                    let delta = (w >> 16) as i16;
                    if delta != 0 {
                        PostMessageW(
                            state.owner,
                            BAR_SCROLL,
                            usize::from(delta < 0),
                            panel.monitor as isize,
                        );
                    }
                    return Some(0);
                }
                if state.slots(width, panel.monitor).into_iter().any(|slot| {
                    slot.item.module == Module::Sound && x >= slot.left && x < slot.right
                }) {
                    if msg == WM_MBUTTONUP {
                        native_status::toggle_mute();
                    } else {
                        native_status::adjust_volume((w >> 16) as i16 as i32 / 120 * 2);
                    }
                    state.ticks = 0;
                    PostMessageW(state.owner, OSD_MESSAGE, 0, 0);
                    PostMessageW(state.owner, WM_TIMER, 1, 0);
                }
                Some(0)
            }
            WM_LBUTTONUP => {
                let Some(panel) = state.panels.iter().find(|p| p.hwnd == hwnd) else {
                    return Some(0);
                };
                let x = (l as i16 as i32) * 96 / panel.dpi as i32;
                let width = (panel.rect.right - panel.rect.left) * 96 / panel.dpi as i32;
                if let Some(slot) = state
                    .slots(width, panel.monitor)
                    .into_iter()
                    .find(|s| x >= s.left && x < s.right)
                {
                    match slot.item.action {
                        208 => {
                            let anchor = RECT {
                                left: panel.rect.left + slot.left * panel.dpi as i32 / 96,
                                top: panel.rect.top,
                                right: panel.rect.left + slot.right * panel.dpi as i32 / 96,
                                bottom: panel.rect.bottom,
                            };
                            if let Some(client) = state.tray_popup.as_mut() {
                                client.click(anchor);
                            }
                        }
                        206 | 207 => {
                            PostMessageW(
                                state.owner,
                                BAR_SCROLL,
                                usize::from(slot.item.action == 207),
                                panel.monitor as isize,
                            );
                        }
                        10 => {
                            PostMessageW(state.owner, BAR_ACTION, 10, panel.monitor as isize);
                        }
                        2000..=10000 => {
                            if let Some(task) = state.tasks.get(slot.item.action - 2000) {
                                tasks::activate(task, true);
                            }
                        }
                        100..=163 => {
                            if !desktops::select(slot.item.action - 100) {
                                log("Desktop selection is unavailable; use Windows Task View.");
                            }
                        }
                        90 => desktops::task_view(),
                        91 => {
                            desktops::windows_panel(b'N' as u16);
                        }
                        92 => {
                            desktops::windows_panel(b'A' as u16);
                        }
                        93 => {
                            desktops::windows_panel(b'A' as u16);
                        }
                        94 => {
                            shell(Path::new("ms-settings:batterysaver"), "");
                        }
                        96 => desktops::start_menu(),
                        98 | 99 => {
                            if let Some(snapshot) = desktops::poll() {
                                let index = if slot.item.action == 98 {
                                    snapshot.current.saturating_sub(1)
                                } else {
                                    (snapshot.current + 1).min(snapshot.ids.len() - 1)
                                };
                                desktops::select(index);
                            } else {
                                desktops::task_view();
                            }
                        }
                        usize::MAX => {}
                        action => {
                            PostMessageW(state.owner, BAR_ACTION, action, 0);
                        }
                    }
                }
                Some(0)
            }
            WM_CLOSE => {
                PostMessageW(state.owner, BAR_ACTION, 3, 0);
                Some(0)
            }
            _ => None,
        }
    });
    handled.unwrap_or_else(|| DefWindowProcW(hwnd, msg, w, l))
}

unsafe fn paint(state: &State, hwnd: HWND) {
    let palette = theme::current();
    let Some(panel) = state.panels.iter().find(|p| p.hwnd == hwnd) else {
        return;
    };
    let mut ps: PAINTSTRUCT = zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    let mut bounds: RECT = zeroed();
    GetClientRect(hwnd, &mut bounds);
    let dc = CreateCompatibleDC(screen);
    let bitmap = CreateCompatibleBitmap(screen, bounds.right.max(1), bounds.bottom.max(1));
    let previous_bitmap = SelectObject(dc, bitmap);
    FillRect(dc, &bounds, state.brush);
    let old_font = SelectObject(dc, panel.font);
    SetBkMode(dc, TRANSPARENT as i32);
    let scale = panel.dpi as i32;
    let width = bounds.right * 96 / scale;
    let draw = |label: &str, left: i32, right: i32, color: u32, align: u32| {
        if right <= left {
            return;
        }
        let text = wide(label);
        let mut rect = RECT {
            left: left * scale / 96,
            right: right * scale / 96,
            top: 0,
            bottom: bounds.bottom - 1,
        };
        SetTextColor(dc, color);
        DrawTextW(
            dc,
            text.as_ptr(),
            (text.len() - 1) as i32,
            &mut rect,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | align,
        );
    };
    for slot in state.slots(width, panel.monitor) {
        if slot.item.module == Module::Divider {
            let line = CreateSolidBrush(palette.border);
            let x = (slot.left + slot.right) / 2 * scale / 96;
            let r = RECT {
                left: x,
                right: x + 1,
                top: 8 * scale / 96,
                bottom: bounds.bottom - 8 * scale / 96,
            };
            FillRect(dc, &r, line);
            DeleteObject(line);
        } else {
            if slot.item.active {
                let brush = CreateSolidBrush(palette.surface);
                let r = RECT {
                    left: slot.left * scale / 96,
                    right: slot.right * scale / 96,
                    top: 3 * scale / 96,
                    bottom: bounds.bottom - 3 * scale / 96,
                };
                FillRect(dc, &r, brush);
                DeleteObject(brush);
                // 0.20: colour alone (1.27:1 tint) did not mark the current workspace or task.
                // Like the Windows taskbar, an accent line under it does, also in grayscale.
                if matches!(slot.item.module, Module::Workspaces | Module::Tasks) {
                    let quarter = (r.right - r.left) / 4;
                    let mark = RECT {
                        left: r.left + quarter,
                        right: r.right - quarter,
                        top: r.bottom - 2 * scale / 96,
                        bottom: r.bottom,
                    };
                    let accent = CreateSolidBrush(palette.accent);
                    FillRect(dc, &mark, accent);
                    DeleteObject(accent);
                }
            }
            let mut text_left = slot.left;
            if slot.item.icon && slot.item.module == Module::Tasks && slot.item.action >= 2000 {
                if let Some(task) = state.tasks.get(slot.item.action - 2000) {
                    let size = 16 * scale / 96;
                    DrawIconEx(
                        dc,
                        (slot.left + 5) * scale / 96,
                        (bounds.bottom - size) / 2,
                        task.identity.icon,
                        size,
                        size,
                        0,
                        null_mut(),
                        DI_NORMAL,
                    );
                    text_left += 23;
                }
            }
            if slot.item.icon && !(slot.item.module == Module::Tasks && slot.item.action >= 2000) {
                let size = 16 * scale / 96;
                bar_icons::draw(
                    dc,
                    (slot.left + 6) * scale / 96,
                    (bounds.bottom - size) / 2,
                    size,
                    slot.item.module,
                    slot.item.action,
                    slot.item.active,
                    if slot.item.active {
                        palette.accent
                    } else {
                        palette.text
                    },
                    if slot.item.module == Module::Power {
                        &state.power
                    } else {
                        &slot.item.tooltip
                    },
                );
                text_left += 23;
            }
            draw(
                &slot.item.label,
                text_left,
                slot.right,
                if slot.item.active || slot.item.module == Module::Menu {
                    palette.accent
                } else if slot.item.module == Module::Tiling {
                    palette.muted
                } else {
                    palette.text
                },
                DT_CENTER,
            );
        }
    }
    let line = CreateSolidBrush(palette.border);
    let rule = RECT {
        left: 0,
        top: bounds.bottom - 1,
        right: bounds.right,
        bottom: bounds.bottom,
    };
    FillRect(dc, &rule, line);
    DeleteObject(line);
    BitBlt(screen, 0, 0, bounds.right, bounds.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old_font);
    SelectObject(dc, previous_bitmap);
    DeleteObject(bitmap);
    DeleteDC(dc);
    EndPaint(hwnd, &ps);
}
unsafe fn network_label() -> String {
    let mut table = null_mut();
    if GetIfTable2(&mut table) != 0 || table.is_null() {
        return "NET ?".into();
    }
    let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
    let mut wired = false;
    let mut wireless = false;
    for row in rows {
        if row.OperStatus == IfOperStatusUp {
            wired |= row.Type == MIB_IF_TYPE_ETHERNET;
            wireless |= row.Type == IF_TYPE_IEEE80211;
        }
    }
    FreeMibTable(table as _);
    if wireless {
        "WI-FI"
    } else if wired {
        "ETH"
    } else {
        "OFFLINE"
    }
    .into()
}
unsafe fn power_label() -> String {
    let mut power: SYSTEM_POWER_STATUS = zeroed();
    if GetSystemPowerStatus(&mut power) == 0 {
        return "PWR ?".into();
    }
    if power.BatteryFlag & 128 != 0 {
        return if power.ACLineStatus == 1 { "AC" } else { "PWR" }.into();
    }
    if power.BatteryLifePercent > 100 {
        return "BAT ?".into();
    }
    format!(
        "{}{}%",
        if power.ACLineStatus == 1 { "+" } else { "" },
        power.BatteryLifePercent
    )
}
