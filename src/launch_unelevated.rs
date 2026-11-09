//! An elevated (Admin) instance starts apps with Explorer's normal token, not its own:
//! it runs `OmarchyOverlay.exe --shell-open <path> <arguments> <directory>` as a child of
//! Explorer, and that helper opens the item and exits.
use std::path::{Path, PathBuf};

const FLAG: &str = "--shell-open";

/// One argument quoted so the Windows command-line parser gives it back unchanged.
pub fn quote(text: &str) -> String {
    let mut out = String::from('"');
    let mut slashes = 0;
    for c in text.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

pub fn helper_command_line(
    exe: &Path,
    path: &Path,
    arguments: &str,
    directory: Option<&Path>,
) -> String {
    let directory = directory.map_or(String::new(), |d| d.to_string_lossy().into_owned());
    [
        exe.to_string_lossy().as_ref(),
        FLAG,
        path.to_string_lossy().as_ref(),
        arguments,
        &directory,
    ]
    .map(quote)
    .join(" ")
}

/// The helper's arguments after the exe name, or None when this is not a helper launch.
pub fn parse_helper_args(args: &[String]) -> Option<(PathBuf, String, Option<PathBuf>)> {
    match args {
        [flag, path, arguments, directory] if flag == FLAG => Some((
            PathBuf::from(path),
            arguments.clone(),
            (!directory.is_empty()).then(|| PathBuf::from(directory)),
        )),
        _ => None,
    }
}

#[cfg(windows)]
pub fn process_elevated() -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Security::*;
    use windows_sys::Win32::System::Threading::*;
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut length = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as _,
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut length,
        ) != 0;
        CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

/// Starts the helper with Explorer as its parent, so it gets Explorer's token.
#[cfg(windows)]
pub unsafe fn shell_unelevated(path: &Path, arguments: &str, directory: Option<&Path>) -> bool {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetShellWindow, GetWindowThreadProcessId};
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let shell = GetShellWindow();
    let mut pid = 0;
    if shell.is_null() || GetWindowThreadProcessId(shell, &mut pid) == 0 {
        return false;
    }
    let parent: HANDLE = OpenProcess(PROCESS_CREATE_PROCESS, 0, pid);
    if parent.is_null() {
        return false;
    }
    let mut size = 0;
    InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut size);
    let mut buffer = vec![0u8; size];
    let list = buffer.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;
    let mut started = false;
    if InitializeProcThreadAttributeList(list, 1, 0, &mut size) != 0 {
        if UpdateProcThreadAttribute(
            list,
            0,
            PROC_THREAD_ATTRIBUTE_PARENT_PROCESS as usize,
            &parent as *const HANDLE as _,
            size_of::<HANDLE>(),
            null_mut(),
            null(),
        ) != 0
        {
            let mut info: STARTUPINFOEXW = std::mem::zeroed();
            info.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
            info.lpAttributeList = list;
            let mut command: Vec<u16> = helper_command_line(&exe, path, arguments, directory)
                .encode_utf16()
                .chain([0])
                .collect();
            let exe_wide: Vec<u16> = exe
                .as_os_str()
                .to_string_lossy()
                .encode_utf16()
                .chain([0])
                .collect();
            let mut process: PROCESS_INFORMATION = std::mem::zeroed();
            started = CreateProcessW(
                exe_wide.as_ptr(),
                command.as_mut_ptr(),
                null(),
                null(),
                0,
                EXTENDED_STARTUPINFO_PRESENT,
                null(),
                null(),
                &info.StartupInfo,
                &mut process,
            ) != 0;
            if started {
                CloseHandle(process.hThread);
                CloseHandle(process.hProcess);
            }
        }
        DeleteProcThreadAttributeList(list);
    }
    CloseHandle(parent);
    started
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The Windows (MSVC) command-line rules, enough to check `quote`.
    fn split(line: &str) -> Vec<String> {
        let (mut args, mut current, mut quoted, mut slashes) = (vec![], String::new(), false, 0);
        let mut any = false;
        for c in line.chars() {
            match c {
                '\\' => slashes += 1,
                '"' => {
                    current.push_str(&"\\".repeat(slashes / 2));
                    if slashes % 2 == 1 {
                        current.push('"');
                    } else {
                        quoted = !quoted;
                        any = true;
                    }
                    slashes = 0;
                }
                ' ' if !quoted => {
                    current.push_str(&"\\".repeat(slashes));
                    slashes = 0;
                    if any || !current.is_empty() {
                        args.push(std::mem::take(&mut current));
                    }
                    any = false;
                }
                _ => {
                    current.push_str(&"\\".repeat(slashes));
                    slashes = 0;
                    current.push(c);
                }
            }
        }
        current.push_str(&"\\".repeat(slashes));
        if any || !current.is_empty() {
            args.push(current);
        }
        args
    }
    #[test]
    fn helper_arguments_round_trip() {
        let exe = Path::new(r"C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe");
        for (path, arguments, directory) in [
            (
                r"C:\Program Files\App\app.exe",
                r#"--open "a b.txt" C:\x\"#,
                Some(r"C:\Work Dir\"),
            ),
            (r"notepad.exe", "", None),
            (r#"C:\q"uote\"#, r#"\\server\share "#, Some(r"D:\")),
        ] {
            let line =
                helper_command_line(exe, Path::new(path), arguments, directory.map(Path::new));
            let args = split(&line);
            assert_eq!(args[0], exe.to_string_lossy());
            assert_eq!(
                parse_helper_args(&args[1..]),
                Some((
                    PathBuf::from(path),
                    arguments.to_string(),
                    directory.map(PathBuf::from)
                )),
                "{line}"
            );
        }
        assert_eq!(parse_helper_args(&["--recover".into()]), None);
        // Restart forwards its own arguments this way; a trailing backslash must survive.
        for argument in [r"D:\cfg\", r#"a "b" c"#, ""] {
            assert_eq!(split(&quote(argument)), [argument]);
        }
    }
}
