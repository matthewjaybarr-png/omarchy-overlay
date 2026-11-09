use super::*;
pub(super) const INSTANCE: &str = "Local\\OmarchyOverlay.0.1";
pub(super) struct Lock(HANDLE);
impl Lock {
    pub(super) unsafe fn acquire() -> Result<Self, String> {
        let handle = CreateMutexW(null(), 0, wide("Local\\OmarchyOverlay.Recovery.v1").as_ptr());
        if handle.is_null() {
            return Err("Cannot create the recovery lock.".into());
        }
        let result = WaitForSingleObject(handle, 10000);
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            CloseHandle(handle);
            return Err("Another recovery operation is still running. Try again shortly.".into());
        }
        Ok(Self(handle))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Position {
    hwnd: usize,
    pid: u32,
    marker: usize,
    normal: [i32; 4],
    flags: u32,
    show: u32,
    min: [i32; 2],
    max: [i32; 2],
}
fn position_path(directory: &Path) -> PathBuf {
    directory.join("placement-restore.json")
}
pub(super) fn save_positions(
    directory: &Path,
    saved: &HashMap<isize, Saved>,
) -> Result<(), String> {
    let path = position_path(directory);
    if saved.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let records: Vec<_> = saved
        .iter()
        .map(|(&hwnd, s)| Position {
            hwnd: hwnd as usize,
            pid: s.pid,
            marker: s.marker,
            normal: [
                s.normal_screen.left,
                s.normal_screen.top,
                s.normal_screen.right,
                s.normal_screen.bottom,
            ],
            flags: s.placement.flags,
            show: s.placement.showCmd,
            min: [s.placement.ptMinPosition.x, s.placement.ptMinPosition.y],
            max: [s.placement.ptMaxPosition.x, s.placement.ptMaxPosition.y],
        })
        .collect();
    write_json_atomic(&path, &records)
}
unsafe fn recover_positions(directory: &Path) -> Result<usize, String> {
    let path = position_path(directory);
    if !path.exists() {
        return Ok(0);
    }
    let records: Vec<Position> = serde_json::from_str(&profile::read_text(&path)?)
        .map_err(|e| format!("Invalid placement recovery data: {e}"))?;
    if records.len() > 4096 {
        return Err("Placement recovery data exceeds the supported limit.".into());
    }
    if records.iter().any(|record| {
        ![
            SW_SHOWNORMAL as u32,
            SW_SHOWMAXIMIZED as u32,
            SW_SHOWMINIMIZED as u32,
            SW_RESTORE as u32,
        ]
        .contains(&record.show)
            || record.normal[2] <= record.normal[0]
            || record.normal[3] <= record.normal[1]
            || record.flags & !7 != 0
    }) {
        return Err("Invalid saved window placement; records retained.".into());
    }
    let count = records.len();
    let mut remaining = HashMap::new();
    for record in records {
        let mut placement: WINDOWPLACEMENT = zeroed();
        placement.length = size_of::<WINDOWPLACEMENT>() as u32;
        placement.flags = record.flags;
        placement.showCmd = record.show;
        placement.ptMinPosition = POINT {
            x: record.min[0],
            y: record.min[1],
        };
        placement.ptMaxPosition = POINT {
            x: record.max[0],
            y: record.max[1],
        };
        let saved = Saved {
            placement,
            pid: record.pid,
            marker: record.marker,
            normal_screen: RECT {
                left: record.normal[0],
                top: record.normal[1],
                right: record.normal[2],
                bottom: record.normal[3],
            },
        };
        if !saved.restore(record.hwnd as HWND) {
            remaining.insert(record.hwnd as isize, saved);
        }
    }
    save_positions(directory, &remaining)?;
    if remaining.is_empty() {
        Ok(count)
    } else {
        Err("Some windows did not respond to placement recovery. Records kept for retry.".into())
    }
}
#[derive(Default, Serialize)]
pub(super) struct Report {
    pub placement_records: usize,
    pub opacity_recovered: bool,
    pub taskbar_recovered: bool,
    pub errors: Vec<String>,
}
pub(super) unsafe fn recover(directory: &Path) -> Report {
    let pending = directory.join("tray-popup-restore.json").exists()
        || position_path(directory).exists()
        || directory.join("opacity-restore.json").exists()
        || directory.join("taskbar-restore.json").exists()
        || directory.join("taskbar-visibility-restore.json").exists()
        || directory.join("window-theme-restore.json").exists()
        || directory.join("windows-colors-restore.json").exists()
        || directory.join("windows-effects-restore.json").exists()
        || directory.join("windows-accent-restore.json").exists()
        || directory.join("wallpaper-restore.json").exists()
        || directory.join("terminal-appearance-restore.json").exists();
    let mut report = Report::default();
    // Restore the usable desktop area first, then translate saved screen positions.
    match taskbar_visibility::recover(directory) {
        Ok(changed) => report.taskbar_recovered = changed,
        Err(e) => report.errors.push(e),
    }
    match bar::recover_taskbar(directory) {
        Ok(changed) => report.taskbar_recovered |= changed,
        Err(e) => report.errors.push(e),
    }
    if let Err(e) = tray_popup::recover(directory) {
        report.errors.push(e);
    }
    match recover_positions(directory) {
        Ok(count) => report.placement_records = count,
        Err(e) => report.errors.push(e),
    }
    if let Err(error) = window_theme::recover(directory) {
        report.errors.push(error);
    }
    if let Err(error) = appearance_effects::restore(directory) {
        report.errors.push(error);
    }
    let opacity_pending = directory.join("opacity-restore.json").exists();
    let mut engine = opacity::Engine::new(directory);
    match engine.restore_all() {
        Ok(()) => report.opacity_recovered = opacity_pending,
        Err(e) => report.errors.push(e),
    }
    if pending {
        if let Err(e) = write_json_atomic(&directory.join("recovery-report.json"), &report) {
            log(&e);
        }
    }
    for error in &report.errors {
        log(error);
    }
    report
}
unsafe fn creation_time(process: HANDLE) -> Option<u64> {
    let (mut created, mut exited, mut kernel, mut user): (FILETIME, FILETIME, FILETIME, FILETIME) =
        (zeroed(), zeroed(), zeroed(), zeroed());
    (GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user) != 0)
        .then_some((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}
fn ready_name(pid: u32, created: u64) -> String {
    format!("Local\\OmarchyOverlay.GuardReady.{pid}.{created}")
}
pub(super) unsafe fn spawn_watchdog() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let created = creation_time(GetCurrentProcess())
        .ok_or("Cannot read the controller's process identity.")?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let ready = CreateEventW(
        null(),
        1,
        0,
        wide(&ready_name(GetCurrentProcessId(), created)).as_ptr(),
    );
    if ready.is_null() {
        return Err("Cannot create the recovery guard handshake.".into());
    }
    let result = std::process::Command::new(executable)
        .args([
            "--watchdog",
            &GetCurrentProcessId().to_string(),
            &created.to_string(),
        ])
        .creation_flags(0x08000000)
        .spawn();
    if let Err(error) = result {
        CloseHandle(ready);
        return Err(format!("Cannot start the recovery guard: {error}"));
    }
    let result = WaitForSingleObject(ready, 3000);
    CloseHandle(ready);
    if result == WAIT_OBJECT_0 {
        Ok(())
    } else {
        Err("Recovery guard did not become ready; no desktop changes were made.".into())
    }
}
pub(super) unsafe fn watchdog(pid: u32, created: u64, directory: &Path) {
    let process = OpenProcess(
        PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        pid,
    );
    if !process.is_null() {
        if creation_time(process) != Some(created) {
            CloseHandle(process);
            return;
        }
        let ready = OpenEventW(
            EVENT_MODIFY_STATE,
            0,
            wide(&ready_name(pid, created)).as_ptr(),
        );
        if !ready.is_null() {
            SetEvent(ready);
            CloseHandle(ready);
        }
        WaitForSingleObject(process, INFINITE);
        CloseHandle(process);
    }
    let Ok(_lock) = Lock::acquire() else {
        log("Recovery guard could not acquire the recovery lock.");
        return;
    };
    let instance = CreateMutexW(null(), 0, wide(INSTANCE).as_ptr());
    if instance.is_null() {
        return;
    }
    if GetLastError() == ERROR_ALREADY_EXISTS {
        CloseHandle(instance);
        return;
    }
    recover(directory);
    CloseHandle(instance);
}
