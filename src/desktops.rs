//! Omarchy Overlay's own workspaces: which one is shown and which hold windows. The windows
//! themselves are parked and brought back by `windows::workspaces`. Windows' virtual desktops are
//! only read, so the overlay's own windows can follow the one in use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub ids: Vec<[u8; 16]>,
    pub current: usize,
}
pub fn decode(ids: &[u8], current: &[u8]) -> Option<Snapshot> {
    if ids.is_empty() || !ids.len().is_multiple_of(16) || ids.len() > 16 * 64 || current.len() != 16
    {
        return None;
    }
    let ids: Vec<[u8; 16]> = ids.as_chunks::<16>().0.to_vec();
    if ids.contains(&[0; 16]) || ids.iter().enumerate().any(|(i, id)| ids[..i].contains(id)) {
        return None;
    }
    let current = ids.iter().position(|id| id.as_slice() == current)?;
    Some(Snapshot { ids, current })
}
impl Snapshot {
    pub fn next(&self) -> Option<usize> {
        (self.ids.len() > 1 && self.current < self.ids.len())
            .then(|| (self.current + 1) % self.ids.len())
    }
}
/// Workspaces are numbered 1 to 10, as in Omarchy.
pub const LIMIT: usize = 10;
/// A stable id per workspace, so layout choices saved by workspace survive a restart.
pub fn workspace_id(index: usize) -> [u8; 16] {
    let mut id = *b"omarchy-ws-00000";
    id[15] = index as u8 + 1;
    id
}
/// The workspaces to show: Omarchy's five, plus any further one holding windows or in use.
/// Bit n of `occupied` is workspace n + 1.
pub fn shown(occupied: u16, current: usize) -> Snapshot {
    let count = 5.max(16 - occupied.leading_zeros() as usize).max(current + 1);
    Snapshot {
        ids: (0..count).map(workspace_id).collect(),
        current,
    }
}
#[cfg(windows)]
mod native {
    use super::*;
    use crate::windows::log;
    use std::{
        cell::RefCell,
        ffi::c_void,
        mem::{size_of, zeroed},
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::HWND,
        System::Threading::GetCurrentProcessId,
        UI::{Input::KeyboardAndMouse::*, Shell::VirtualDesktopManager},
    };
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    use windows_sys::{
        core::GUID,
        Win32::System::{Com::*, Registry::*, RemoteDesktop::ProcessIdToSessionId},
    };
    const ROOT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops";
    #[derive(Default)]
    struct Controller {
        current: usize,
        /// The workspace shown before the current one.
        former: Option<usize>,
        occupied: u16,
    }
    thread_local! {static DESKTOPS:RefCell<Controller>=RefCell::new(Controller::default());}
    unsafe fn binary(key: &str, value: &str) -> Option<Vec<u8>> {
        let mut size = 0;
        let key = wide(key);
        let value = wide(value);
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_BINARY,
            null_mut(),
            null_mut(),
            &mut size,
        ) != 0
            || size == 0
            || size > 1024
        {
            return None;
        }
        let mut bytes = vec![0u8; size as usize];
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_BINARY,
            null_mut(),
            bytes.as_mut_ptr() as _,
            &mut size,
        ) != 0
        {
            return None;
        }
        bytes.truncate(size as usize);
        Some(bytes)
    }
    unsafe fn read() -> Option<Snapshot> {
        let ids = binary(ROOT, "VirtualDesktopIDs")?;
        let mut session = 0;
        ProcessIdToSessionId(GetCurrentProcessId(), &mut session);
        let session_key=format!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SessionInfo\\{session}\\VirtualDesktops");
        // Windows versions differ. Only accept a current ID that is in the observed list.
        for key in [ROOT, &session_key] {
            if let Some(current) = binary(key, "CurrentVirtualDesktop") {
                if let Some(s) = decode(&ids, &current) {
                    return Some(s);
                }
            }
        }
        None
    }
    unsafe fn modifiers_released() -> bool {
        [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .all(|&key| GetAsyncKeyState(key as i32) >= 0)
    }
    unsafe fn chord(keys: &[u16]) -> bool {
        if !modifiers_released() {
            return false;
        }
        let mut inputs = Vec::new();
        for (key, up) in keys
            .iter()
            .map(|&k| (k, false))
            .chain(keys.iter().rev().map(|&k| (k, true)))
        {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                        ..zeroed()
                    },
                },
            });
        }
        let count = SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        );
        if count != inputs.len() as u32 {
            // Release only the keys from this synthetic chord after a partial send.
            let releases: Vec<_> = keys
                .iter()
                .rev()
                .map(|&key| INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: key,
                            dwFlags: KEYEVENTF_KEYUP,
                            ..zeroed()
                        },
                    },
                })
                .collect();
            SendInput(
                releases.len() as u32,
                releases.as_ptr(),
                size_of::<INPUT>() as i32,
            );
            return false;
        }
        true
    }
    pub unsafe fn native_control(kind: u32) -> bool {
        match kind {
            0 => chord(&[VK_LWIN, b'V' as u16]),
            1 => chord(&[VK_LWIN, VK_SHIFT, b'S' as u16]),
            2 => chord(&[VK_LWIN, b'X' as u16]),
            3 => chord(&[VK_LWIN, b'A' as u16]),
            4 => chord(&[VK_LWIN, b'N' as u16]),
            5 => chord(&[VK_LWIN, VK_OEM_PERIOD]),
            6 => chord(&[VK_LWIN, VK_TAB]),
            // Universal clipboard, sent once Super is up so it does not merge into the chord.
            7 => chord(&[VK_CONTROL, b'C' as u16]),
            8 => chord(&[VK_CONTROL, b'V' as u16]),
            9 => chord(&[VK_CONTROL, b'X' as u16]),
            10 => chord(&[VK_CONTROL, b'A' as u16]),
            11 => chord(&[VK_CONTROL, VK_SHIFT, b'C' as u16]),
            12 => chord(&[VK_CONTROL, VK_SHIFT, b'V' as u16]),
            _ => true,
        }
    }
    pub unsafe fn start_menu() {
        if !chord(&[VK_CONTROL, VK_ESCAPE]) {
            log("Windows Start shortcut was not sent; release modifiers and try again.");
        }
    }
    pub unsafe fn windows_panel(key: u16) {
        if [b'A' as u16, b'N' as u16].contains(&key) && !chord(&[VK_LWIN, key]) {
            log("Windows panel shortcut was not sent; release modifier keys and try again.");
        }
    }
    pub unsafe fn task_view() {
        if !chord(&[VK_LWIN, VK_TAB]) {
            log("Task View shortcut could not be sent; release modifier keys and try again.");
        }
    }
    /// Makes workspace `index` (from 0) the shown one; the next tiling pass moves the windows.
    pub unsafe fn select(index: usize) -> bool {
        DESKTOPS.with(|c| {
            let mut c = c.borrow_mut();
            if index >= LIMIT {
                return false;
            }
            if c.current != index {
                c.former = Some(c.current);
                c.current = index;
            }
            true
        })
    }
    pub unsafe fn select_former() -> bool {
        match DESKTOPS.with(|c| c.borrow().former) {
            Some(former) => select(former),
            None => false,
        }
    }
    pub unsafe fn cycle_next() -> bool {
        match snapshot().and_then(|s| s.next()) {
            Some(next) => select(next),
            None => false,
        }
    }
    pub unsafe fn poll() -> Option<Snapshot> {
        snapshot()
    }
    pub fn snapshot() -> Option<Snapshot> {
        Some(DESKTOPS.with(|c| shown(c.borrow().occupied, c.borrow().current)))
    }
    pub fn current() -> usize {
        DESKTOPS.with(|c| c.borrow().current)
    }
    pub fn set_occupied(occupied: u16) {
        DESKTOPS.with(|c| c.borrow_mut().occupied = occupied)
    }
    // Public IVirtualDesktopManager is used only to move Omarchy Overlay's own windows to
    // the Windows desktop in use, so the bar follows native keyboard/Task View changes.
    #[repr(C)]
    struct Vtable {
        query: usize,
        add_ref: usize,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        is_current: usize,
        get_id: usize,
        move_window: unsafe extern "system" fn(*mut c_void, HWND, *const GUID) -> i32,
    }
    pub unsafe fn follow(hwnd: HWND) {
        let Some(s) = read() else {
            return;
        };
        let bytes = s.ids[s.current];
        let guid = GUID {
            data1: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            data2: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
            data3: u16::from_le_bytes(bytes[6..8].try_into().unwrap()),
            data4: bytes[8..16].try_into().unwrap(),
        };
        let init = CoInitializeEx(null(), COINIT_APARTMENTTHREADED as u32);
        let iid = GUID::from_u128(0xa5cd92ff_29be_454c_8d04_d82879fb3f1b);
        let mut object: *mut c_void = null_mut();
        if CoCreateInstance(
            &VirtualDesktopManager,
            null_mut(),
            CLSCTX_INPROC_SERVER,
            &iid,
            &mut object,
        ) >= 0
            && !object.is_null()
        {
            let table = &**(object as *const *const Vtable);
            let result = (table.move_window)(object, hwnd, &guid);
            if result < 0 {
                log("Omarchy Overlay window could not follow the active Windows desktop.");
            }
            (table.release)(object);
        }
        if init >= 0 {
            CoUninitialize();
        }
    }
}
#[cfg(windows)]
pub use native::*;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_snapshot_requires_matching_unique_ids() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        let ids = [a, b].concat();
        let s = decode(&ids, &b).unwrap();
        assert_eq!(s.current, 1);
        assert!(decode(&ids, &[3; 16]).is_none());
        assert!(decode(&ids[..17], &a).is_none());
        assert!(decode(&[a, a].concat(), &a).is_none());
        assert!(decode(&[0; 16], &[0; 16]).is_none());
    }
    #[test]
    fn five_workspaces_show_plus_any_further_one_in_use() {
        assert_eq!(shown(0, 0).ids.len(), 5);
        assert_eq!(shown(1 << 7, 0).ids.len(), 8);
        let s = shown(0b11, 9);
        assert_eq!((s.ids.len(), s.current), (10, 9));
        assert_ne!(workspace_id(0), workspace_id(1));
    }
    #[test]
    fn cycling_wraps_only_across_observed_desktops() {
        let mut s = Snapshot {
            ids: vec![[1; 16], [2; 16], [3; 16]],
            current: 0,
        };
        assert_eq!(s.next(), Some(1));
        s.current = 2;
        assert_eq!(s.next(), Some(0));
        s.ids.truncate(1);
        s.current = 0;
        assert_eq!(s.next(), None);
        s.ids.clear();
        assert_eq!(s.next(), None);
    }
}
