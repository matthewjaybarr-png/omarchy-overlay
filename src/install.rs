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

pub fn uninstall_values(dir: &Path, version: &str) -> Vec<(&'static str, String)> {
    // Built as text so the Windows separator is used on every build host.
    let exe = format!("{}\\OmarchyOverlay.exe", dir.display());
    vec![
        ("DisplayName", TASK.into()),
        ("DisplayVersion", version.into()),
        ("InstallLocation", dir.display().to_string()),
        ("DisplayIcon", exe.clone()),
        ("UninstallString", format!("\"{exe}\" --uninstall")),
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

/// Relaunches this exe elevated with `arguments`; false when the prompt was refused.
#[cfg(windows)]
pub unsafe fn elevate(arguments: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    ShellExecuteW(
        std::ptr::null_mut(),
        wide("runas").as_ptr(),
        wide(&exe.to_string_lossy()).as_ptr(),
        wide(arguments).as_ptr(),
        std::ptr::null(),
        SW_SHOWNORMAL,
    ) as isize
        > 32
}

/// Copies the program and registers the logon task, shortcut and Apps entry. Needs admin.
#[cfg(windows)]
pub fn install(user: &str) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let source = exe.parent().ok_or("No program folder.")?;
    let dir = install_dir();
    if !is_installed(&exe) {
        std::fs::create_dir_all(dir.join("fonts")).map_err(|e| e.to_string())?;
        let copy = |from: &Path, to: &Path| {
            std::fs::copy(from, to)
                .map(drop)
                .map_err(|e| format!("Could not copy {}: {e}", from.display()))
        };
        copy(&exe, &dir.join("OmarchyOverlay.exe"))?;
        if source.join("RECOVER.cmd").is_file() {
            copy(&source.join("RECOVER.cmd"), &dir.join("RECOVER.cmd"))?;
        }
        if let Ok(fonts) = std::fs::read_dir(source.join("fonts")) {
            for font in fonts.flatten().filter(|f| f.path().is_file()) {
                copy(&font.path(), &dir.join("fonts").join(font.file_name()))?;
            }
        }
    }
    let installed = dir.join("OmarchyOverlay.exe");
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
    for (name, value) in uninstall_values(&dir, env!("CARGO_PKG_VERSION")) {
        let kind = if name.starts_with("No") { "REG_DWORD" } else { "REG_SZ" };
        run(
            "reg.exe",
            &["add", UNINSTALL_KEY, "/v", name, "/t", kind, "/d", &value, "/f"].map(String::from),
        )?;
    }
    Ok(dir)
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

/// Deletes the install folder a few seconds after this process exits (this exe may be in it).
#[cfg(windows)]
pub fn remove_folder_after_exit() {
    use std::os::windows::process::CommandExt;
    let dir = install_dir();
    if dir.exists() {
        let _ = std::process::Command::new(system("cmd.exe"))
            .raw_arg(format!("/c ping -n 3 127.0.0.1>nul & rmdir /s /q \"{}\"", dir.display()))
            .current_dir(std::env::temp_dir())
            .creation_flags(0x0800_0000 | 0x0000_0008) // CREATE_NO_WINDOW | DETACHED_PROCESS
            .spawn();
    }
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
    fn apps_entry_values() {
        let dir = Path::new(r"C:\Program Files\OmarchyOverlay");
        let values = uninstall_values(dir, "0.2.0");
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
    }
}
