//! Launch Rainmeter's registered removal wizard without a command shell or silent flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub executable: String,
    pub arguments: String,
}
pub fn parse_command(value: &str) -> Option<Command> {
    if value.chars().any(char::is_control) {
        return None;
    }
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let (executable, arguments) = if let Some(rest) = value.strip_prefix('"') {
        let end = rest.find('"')?;
        (&rest[..end], rest[end + 1..].trim())
    } else {
        // Rainmeter 4 wrote an unquoted path containing spaces; never split at the first space.
        let end = value.to_ascii_lowercase().find(".exe")? + 4;
        (&value[..end], value[end..].trim())
    };
    if executable.contains('"')
        || !((executable.as_bytes().get(1) == Some(&b':')
            && executable
                .as_bytes()
                .get(2)
                .is_some_and(|c| *c == b'\\' || *c == b'/'))
            || executable.starts_with("\\\\"))
    {
        return None;
    }
    let name = executable.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
    match name.as_str() {
        "uninst.exe" | "uninstall.exe" if arguments.is_empty() => {}
        "rainmeter.exe" if arguments.eq_ignore_ascii_case("/Uninstall") => {}
        _ => return None,
    }
    Some(Command {
        executable: executable.into(),
        arguments: arguments.into(),
    })
}
#[cfg(all(windows, not(test)))]
mod native {
    use super::*;
    use std::{mem::size_of, path::PathBuf, ptr::null_mut};
    use windows_sys::Win32::System::Registry::*;
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    unsafe fn value(root: HKEY, key: &str, name: &str, view: u32) -> Option<String> {
        let mut bytes = 0;
        let key = wide(key);
        let name = wide(name);
        let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | view;
        if RegGetValueW(
            root,
            key.as_ptr(),
            name.as_ptr(),
            flags,
            null_mut(),
            null_mut(),
            &mut bytes,
        ) != 0
            || bytes == 0
            || bytes > 32768
            || bytes % 2 != 0
        {
            return None;
        }
        let mut text = vec![0u16; bytes as usize / size_of::<u16>()];
        if RegGetValueW(
            root,
            key.as_ptr(),
            name.as_ptr(),
            flags,
            null_mut(),
            text.as_mut_ptr() as _,
            &mut bytes,
        ) != 0
        {
            return None;
        }
        let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
        String::from_utf16(&text[..end]).ok()
    }
    pub fn discover() -> Option<Command> {
        unsafe {
            const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Rainmeter";
            for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
                for view in [RRF_SUBKEY_WOW6464KEY, RRF_SUBKEY_WOW6432KEY] {
                    if value(root, KEY, "DisplayName", view)
                        .is_none_or(|s| !s.trim().eq_ignore_ascii_case("Rainmeter"))
                    {
                        continue;
                    }
                    if let Some(command) =
                        value(root, KEY, "UninstallString", view).and_then(|s| parse_command(&s))
                    {
                        if PathBuf::from(&command.executable).is_file() {
                            return Some(command);
                        }
                    }
                }
            }
            // Recover older installs whose registration was lost, without guessing the new /Uninstall verb.
            for variable in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(base) = std::env::var_os(variable) {
                    let folder = PathBuf::from(base).join("Rainmeter");
                    let path = folder.join("uninst.exe");
                    if folder.join("Rainmeter.exe").is_file() && path.is_file() {
                        return Some(Command {
                            executable: path.to_string_lossy().into(),
                            arguments: String::new(),
                        });
                    }
                }
            }
            None
        }
    }
}
#[cfg(all(windows, not(test)))]
pub use native::discover;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_rainmeter_4_and_5_commands_without_silent_flags() {
        assert_eq!(
            parse_command(r#"C:\Program Files\Rainmeter\uninst.exe"#)
                .unwrap()
                .executable,
            r#"C:\Program Files\Rainmeter\uninst.exe"#
        );
        assert!(
            parse_command(r#""D:\Desktop tools\Rainmeter\Rainmeter.exe" /Uninstall"#).is_some()
        );
        assert!(parse_command(r#""C:\Program Files\Rainmeter\uninst.exe""#).is_some());
    }
    #[test]
    fn rejects_shells_silent_flags_and_relative_paths() {
        for value in [
            r#"cmd.exe /c something"#,
            r#"C:\Rainmeter\uninst.exe /S"#,
            r#"C:\Rainmeter\Rainmeter.exe /Uninstall /S"#,
            r#"uninst.exe"#,
            r#""C:\Rainmeter\uninst.exe" & calc.exe"#,
            r#"C:\Other\program.exe"#,
            "C:\\Rainmeter\\uninst.exe\n",
        ] {
            assert!(parse_command(value).is_none(), "{value}");
        }
    }
}
