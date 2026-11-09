//! Admin edition: installs to %ProgramFiles%\OmarchyOverlay with an elevated logon task,
//! a Start menu shortcut and an Apps entry; uninstall removes all of it.
use std::path::{Path, PathBuf};

pub const TASK: &str = "Omarchy Overlay";
const FOLDER: &str = "OmarchyOverlay";
const UNINSTALL_KEY: &str = r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\OmarchyOverlay";

/// (show "Install (admin)…", show "Uninstall…") for the tray menu.
pub fn tray_install_items(marker: bool, installed: bool) -> (bool, bool) {
    (marker && !installed, installed)
}

pub fn schtasks_create_args(exe: &Path, user: &str) -> Vec<String> {
    let exe = format!("\"{}\"", exe.display());
    [
        "/Create", "/F", "/TN", TASK, "/SC", "ONLOGON", "/RL", "HIGHEST", "/RU", user, "/IT",
        "/TR", &exe,
    ]
    .map(String::from)
    .to_vec()
}

/// schtasks keeps Task Scheduler defaults: stop after 72 hours, and never start or keep running
/// on battery. This PowerShell clears all three on the existing task.
pub fn task_settings_script() -> String {
    format!(
        "$t=Get-ScheduledTask -TaskName '{TASK}';$t.Settings.ExecutionTimeLimit='PT0S';$t.Settings.DisallowStartIfOnBatteries=$false;$t.Settings.StopIfGoingOnBatteries=$false;Set-ScheduledTask -InputObject $t|Out-Null"
    )
}

/// Apps entry values stored as numbers.
pub const DWORD_VALUES: [&str; 3] = ["NoModify", "NoRepair", "EstimatedSize"];

pub fn uninstall_values(dir: &Path, version: &str, size_kib: u64, date: &str) -> Vec<(&'static str, String)> {
    // Built as text so the Windows separator is used on every build host.
    let exe = format!("{}\\OmarchyOverlay.exe", dir.display());
    vec![
        ("DisplayName", TASK.into()),
        ("DisplayVersion", version.into()),
        ("InstallLocation", dir.display().to_string()),
        ("DisplayIcon", exe.clone()),
        ("UninstallString", format!("\"{exe}\" --uninstall")),
        ("QuietUninstallString", format!("\"{exe}\" --uninstall --quiet")),
        ("InstallDate", date.into()),
        ("EstimatedSize", size_kib.min(u64::from(u32::MAX)).to_string()),
        ("NoModify", "1".into()),
        ("NoRepair", "1".into()),
    ]
}

pub fn install_dir() -> PathBuf {
    std::env::var_os("ProgramFiles")
        .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from)
        .join(FOLDER)
}

/// True when this exe runs from the install folder.
pub fn is_installed(exe: &Path) -> bool {
    exe.parent().is_some_and(|p| {
        p.to_string_lossy()
            .eq_ignore_ascii_case(&install_dir().to_string_lossy())
    })
}

/// The Admin release ships an `edition-admin` file beside the exe.
pub fn admin_marker(exe: &Path) -> bool {
    exe.parent().is_some_and(|p| p.join("edition-admin").is_file())
}

#[cfg(windows)]
fn shortcut() -> PathBuf {
    std::env::var_os("ProgramData")
        .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from)
        .join(r"Microsoft\Windows\Start Menu\Programs\Omarchy Overlay.lnk")
}

/// A Windows tool's full path. A bare name would also search this exe's folder, which an
/// elevated install from a user-writable folder must never run from.
pub fn system_tool(root: &str, name: &str) -> String {
    let folder = if name == "powershell.exe" { r"System32\WindowsPowerShell\v1.0" } else { "System32" };
    format!("{root}\\{folder}\\{name}")
}
#[cfg(windows)]
fn system(name: &str) -> String {
    system_tool(&std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into()), name)
}
#[cfg(windows)]
fn run(program: &str, args: &[String]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let status = std::process::Command::new(system(program))
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status}).", args.first().map_or("", |a| a)))
    }
}

/// Relaunches this exe elevated with `arguments` and waits for it. The exit code of the
/// elevated copy, or None when the prompt was refused or could not be shown.
#[cfg(windows)]
pub unsafe fn elevate(arguments: &str) -> Option<u32> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let exe = std::env::current_exe().ok()?;
    let (verb, file, parameters) = (wide("runas"), wide(&exe.to_string_lossy()), wide(arguments));
    let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
    info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if ShellExecuteExW(&mut info) == 0 || info.hProcess.is_null() {
        return None;
    }
    WaitForSingleObject(info.hProcess, INFINITE);
    let mut code = 1;
    GetExitCodeProcess(info.hProcess, &mut code);
    CloseHandle(info.hProcess);
    Some(code)
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// A text value of the Apps entry; readable without administrator rights.
#[cfg(windows)]
fn apps_entry(name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let key = wide(UNINSTALL_KEY.trim_start_matches(r"HKLM\"));
    let name = wide(name);
    let mut text = [0u16; 1024];
    let mut bytes = (text.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            text.as_mut_ptr() as _,
            &mut bytes,
        )
    };
    if status != 0 {
        return None;
    }
    let len = (bytes as usize / 2).saturating_sub(1).min(text.len());
    Some(String::from_utf16_lossy(&text[..len])).filter(|t| !t.trim().is_empty())
}
/// The installed version: the Apps entry's, or the install manifest's when the entry is gone.
#[cfg(windows)]
pub fn installed_version() -> Option<String> {
    apps_entry("DisplayVersion").or_else(|| crate::install_files::manifest_version(&install_dir()))
}
/// What this copy's install would do now.
#[cfg(windows)]
pub fn current_plan() -> crate::install_files::Plan {
    crate::install_files::plan(
        installed_version().as_deref(),
        install_dir().join(crate::install_files::EXE).is_file(),
        env!("CARGO_PKG_VERSION"),
    )
}

/// Waits up to `timeout_ms` for every program running from `dir` (other than this one, in
/// any session: an old copy, its recovery guard) to exit. Returns how many still run.
#[cfg(windows)]
pub unsafe fn wait_for_programs_in(dir: &Path, timeout_ms: u32) -> usize {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::System::Threading::*;
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == INVALID_HANDLE_VALUE {
        return 0;
    }
    let mut running = vec![];
    let mut entry: PROCESSENTRY32W = std::mem::zeroed();
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let own = GetCurrentProcessId();
    let mut more = Process32FirstW(snapshot, &mut entry) != 0;
    while more {
        if entry.th32ProcessID != own {
            let process = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                entry.th32ProcessID,
            );
            if !process.is_null() {
                let mut image = [0u16; 1024];
                let mut len = image.len() as u32;
                let ours = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, image.as_mut_ptr(), &mut len) != 0
                    && crate::install_files::in_folder(
                        Path::new(&String::from_utf16_lossy(&image[..len as usize])),
                        dir,
                    );
                if ours {
                    running.push(process);
                } else {
                    CloseHandle(process);
                }
            }
        }
        more = Process32NextW(snapshot, &mut entry) != 0;
    }
    CloseHandle(snapshot);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms.into());
    let mut left = 0;
    for process in running {
        let wait = deadline.saturating_duration_since(std::time::Instant::now()).as_millis() as u32;
        if WaitForSingleObject(process, wait) != 0 {
            left += 1;
        }
        CloseHandle(process);
    }
    left
}

/// Has Windows delete a file that is still in use at the next restart.
#[cfg(windows)]
fn delete_at_restart(path: &Path) {
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
    unsafe {
        MoveFileExW(wide(&path.to_string_lossy()).as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT);
    }
}

/// Today as `YYYYMMDD`, the Apps entry's install date.
#[cfg(windows)]
fn today() -> String {
    let mut time = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut time) };
    format!("{:04}{:02}{:02}", time.wYear, time.wMonth, time.wDay)
}

/// Copies the program (replacing an older or newer copy as a whole, see install_files),
/// then registers the logon task, shortcut and Apps entry. Needs admin. Returns the old
/// files Windows will delete at the next restart, if any were still in use.
#[cfg(windows)]
pub fn install(user: &str) -> Result<crate::install_files::Replaced, String> {
    use crate::install_files as files;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = install_dir();
    let previous_location = apps_entry("InstallLocation");
    let items = files::release_files(&exe);
    let mut report = files::Replaced::default();
    if is_installed(&exe) {
        // Repair from the installed copy: the files are already these.
        files::clear_leftovers(&dir);
    } else {
        // The old copy was asked to exit; its recovery guard follows it within moments.
        let still_running = unsafe { wait_for_programs_in(&dir, 10_000) };
        if still_running > 0 {
            crate::windows::log(&format!(
                "Install: {still_running} program(s) from {} still running; replacing their files aside.",
                dir.display()
            ));
        }
        report = files::replace_files(&dir, &items, env!("CARGO_PKG_VERSION"))?;
        for path in &report.pending_delete {
            delete_at_restart(path);
        }
        for removed in &report.stale_removed {
            crate::windows::log(&format!("Install: removed {removed}, which this version no longer ships."));
        }
    }
    let installed = dir.join(files::EXE);
    run("schtasks.exe", &schtasks_create_args(&installed, user))?;
    run(
        "powershell.exe",
        &["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), task_settings_script()],
    )?;
    let quote = |p: &Path| p.display().to_string().replace('\'', "''");
    let schtasks = system("schtasks.exe");
    run(
        "powershell.exe",
        &[
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            format!(
                "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.Arguments='/Run /TN \"{TASK}\"';$s.IconLocation='{},0';$s.WindowStyle=7;$s.Save()",
                quote(&shortcut()),
                schtasks.replace('\'', "''"),
                quote(&installed)
            ),
        ],
    )?;
    let size = if is_installed(&exe) {
        files::size_kib(&files::release_files(&installed))
    } else {
        files::size_kib(&items)
    };
    for (name, value) in uninstall_values(&dir, env!("CARGO_PKG_VERSION"), size, &today()) {
        let kind = if DWORD_VALUES.contains(&name) { "REG_DWORD" } else { "REG_SZ" };
        run(
            "reg.exe",
            &["add", UNINSTALL_KEY, "/v", name, "/t", kind, "/d", &value, "/f"].map(String::from),
        )?;
    }
    // An install recorded somewhere else (another Program Files) is now replaced by this one.
    if let Some(old) = previous_location.map(PathBuf::from) {
        if is_old_install(&old, &dir) {
            let _ = std::fs::remove_dir_all(&old);
        }
    }
    Ok(report)
}

/// Starts the installed copy through its logon task, elevated without a prompt.
#[cfg(windows)]
pub fn start_task() -> Result<(), String> {
    run("schtasks.exe", &["/Run", "/TN", TASK].map(String::from))
}

/// Removes the task, shortcut, Apps entry and, when given, the settings folder.
/// Returns what could not be removed.
#[cfg(windows)]
pub fn remove(settings: Option<&Path>) -> Vec<String> {
    let mut errors = vec![];
    let _ = run("schtasks.exe", &["/Delete", "/F", "/TN", TASK].map(String::from));
    if shortcut().exists() {
        if let Err(e) = std::fs::remove_file(shortcut()) {
            errors.push(format!("Start menu shortcut: {e}"));
        }
    }
    let _ = run("reg.exe", &["delete", UNINSTALL_KEY, "/f"].map(String::from));
    if let Some(settings) = settings.filter(|s| s.exists()) {
        if let Err(e) = std::fs::remove_dir_all(settings) {
            errors.push(format!("{}: {e}", settings.display()));
        }
    }
    errors
}

/// Deletes the install folder after this process exits (this exe may be in it). Removes
/// what it can now, then retries the rest for 15 seconds from a detached command.
#[cfg(windows)]
pub fn remove_folder_after_exit() {
    use std::os::windows::process::CommandExt;
    let dir = install_dir();
    if !dir.exists() {
        return;
    }
    let own = std::env::current_exe().ok();
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if Some(&path) == own.as_ref() {
            continue;
        }
        let _ = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
    }
    let _ = std::process::Command::new(system("cmd.exe"))
        .raw_arg(remove_folder_command(&dir))
        .current_dir(std::env::temp_dir())
        .creation_flags(0x0800_0000 | 0x0000_0008) // CREATE_NO_WINDOW | DETACHED_PROCESS
        .spawn();
}

/// The detached command that removes the install folder once nothing in it runs.
pub fn remove_folder_command(dir: &Path) -> String {
    let dir = dir.display();
    format!("/c for /l %i in (1,1,15) do @if exist \"{dir}\" (ping -n 2 127.0.0.1>nul & rmdir /s /q \"{dir}\" 2>nul)")
}

/// An earlier install at `old` that this install to `dir` replaces: only a different
/// folder named like ours that holds our program is ever removed.
pub fn is_old_install(old: &Path, dir: &Path) -> bool {
    !old.to_string_lossy().eq_ignore_ascii_case(&dir.to_string_lossy())
        && old
            .file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(FOLDER))
        && old.join(crate::install_files::EXE).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_items_follow_marker_and_install_folder() {
        assert_eq!(tray_install_items(false, false), (false, false));
        assert_eq!(tray_install_items(true, false), (true, false));
        assert_eq!(tray_install_items(true, true), (false, true));
        assert_eq!(tray_install_items(false, true), (false, true));
    }
    #[test]
    fn system_tools_resolve_in_system32_never_beside_the_exe() {
        assert_eq!(system_tool(r"C:\Windows", "reg.exe"), r"C:\Windows\System32\reg.exe");
        assert_eq!(
            system_tool(r"D:\Win", "powershell.exe"),
            r"D:\Win\System32\WindowsPowerShell\v1.0\powershell.exe"
        );
    }
    #[test]
    fn logon_task_runs_highest_for_the_installing_user() {
        let exe = Path::new(r"C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe");
        assert_eq!(
            schtasks_create_args(exe, r"PC\user"),
            [
                "/Create", "/F", "/TN", "Omarchy Overlay", "/SC", "ONLOGON", "/RL", "HIGHEST",
                "/RU", r"PC\user", "/IT", "/TR",
                r#""C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe""#,
            ]
        );
    }
    #[test]
    fn logon_task_never_times_out_or_stops_on_battery() {
        let script = task_settings_script();
        for part in [
            "Get-ScheduledTask -TaskName 'Omarchy Overlay'",
            "ExecutionTimeLimit='PT0S'",
            "DisallowStartIfOnBatteries=$false",
            "StopIfGoingOnBatteries=$false",
            "Set-ScheduledTask -InputObject $t",
        ] {
            assert!(script.contains(part), "{part} missing from {script}");
        }
    }
    #[test]
    fn manifest_version_follows_the_package_version() {
        let expected = format!("version=\"{}.0\"", env!("CARGO_PKG_VERSION"));
        assert!(include_str!("../app.manifest").contains(&expected), "app.manifest needs {expected}");
    }
    #[test]
    fn only_a_separate_folder_holding_our_program_counts_as_an_old_install() {
        let root = std::env::temp_dir().join(format!("overlay-old-install-{}", std::process::id()));
        let old = root.join("Old Program Files").join(FOLDER);
        let dir = root.join("Program Files").join(FOLDER);
        std::fs::create_dir_all(&old).unwrap();
        assert!(!is_old_install(&old, &dir), "no program in it");
        std::fs::write(old.join(crate::install_files::EXE), "x").unwrap();
        assert!(is_old_install(&old, &dir));
        assert!(!is_old_install(&dir, &dir));
        let other = root.join("Something");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join(crate::install_files::EXE), "x").unwrap();
        assert!(!is_old_install(&other, &dir), "a folder with another name is never removed");
        let _ = std::fs::remove_dir_all(&root);
    }
    #[test]
    fn folder_removal_retries_quietly() {
        let command = remove_folder_command(Path::new(r"C:\Program Files\OmarchyOverlay"));
        assert!(command.starts_with("/c for /l %i in (1,1,15)"), "{command}");
        assert_eq!(command.matches(r#""C:\Program Files\OmarchyOverlay""#).count(), 2);
    }
    #[test]
    fn apps_entry_values() {
        let dir = Path::new(r"C:\Program Files\OmarchyOverlay");
        let values = uninstall_values(dir, "0.2.0", 12_345, "20261109");
        let get = |n: &str| values.iter().find(|(k, _)| *k == n).unwrap().1.clone();
        assert_eq!(get("DisplayName"), "Omarchy Overlay");
        assert_eq!(get("DisplayVersion"), "0.2.0");
        assert_eq!(get("InstallLocation"), r"C:\Program Files\OmarchyOverlay");
        assert_eq!(get("DisplayIcon"), r"C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe");
        assert_eq!(
            get("UninstallString"),
            r#""C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe" --uninstall"#
        );
        assert_eq!((get("NoModify"), get("NoRepair")), ("1".into(), "1".into()));
        assert_eq!(
            get("QuietUninstallString"),
            r#""C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe" --uninstall --quiet"#
        );
        assert_eq!((get("InstallDate"), get("EstimatedSize")), ("20261109".into(), "12345".into()));
        for name in DWORD_VALUES {
            assert!(get(name).parse::<u32>().is_ok(), "{name} must be a number");
        }
    }
}
