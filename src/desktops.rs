//! Native Windows desktop observation. Explorer data is read only and optional.
//! Switching uses the documented Windows+Ctrl+Arrow shortcuts, never internal COM interfaces.
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
    use std::time::{Duration, Instant};
    use windows_sys::{
        core::GUID,
        Win32::System::{Com::*, Registry::*, RemoteDesktop::ProcessIdToSessionId},
    };
    const ROOT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VirtualDesktops";
    struct Pending {
        target: [u8; 16],
        sent_from: Option<[u8; 16]>,
        sent_at: Instant,
        deadline: Instant,
    }
    #[derive(Default)]
    struct Controller {
        snapshot: Option<Snapshot>,
        pending: Option<Pending>,
        settle_until: Option<Instant>,
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
    pub unsafe fn select(index: usize) -> bool {
        DESKTOPS.with(|c| {
            let mut c = c.borrow_mut();
            let Some(s) = read() else {
                return false;
            };
            let Some(&target) = s.ids.get(index) else {
                return false;
            };
            c.pending = Some(Pending {
                target,
                sent_from: None,
                sent_at: Instant::now(),
                deadline: Instant::now() + Duration::from_secs(8),
            });
            c.snapshot = Some(s);
            true
        })
    }
    pub unsafe fn cycle_next() -> bool {
        // Read actual Explorer state for each invocation. Never guess the desktop,
        // create desktops or queue extra transitions during an in-flight switch.
        let Some(observed) = read() else {
            return false;
        };
        let in_flight = DESKTOPS.with(|c| {
            c.borrow()
                .pending
                .as_ref()
                .is_some_and(|pending| pending.target != observed.ids[observed.current])
        });
        if in_flight {
            return false;
        }
        // The tiling settle delay protects window geometry after a switch; it
        // must not swallow another cycle once the target desktop is observed.
        let Some(next) = observed.next() else {
            return false;
        };
        select(next)
    }
    pub unsafe fn poll() -> Option<Snapshot> {
        DESKTOPS.with(|cell| {
            let mut controller = cell.borrow_mut();
            let observed = read();
            if controller.snapshot.as_ref().map(|s|s.ids[s.current]) != observed.as_ref().map(|s|s.ids[s.current]) {
                controller.settle_until = Some(Instant::now()+Duration::from_millis(1100));
            }
            controller.snapshot = observed;
            let snapshot = controller.snapshot.clone();
            let Some(pending) = &mut controller.pending else { return snapshot; };
            let mut done = true;
            if let Some(observed) = &snapshot {
                let current = observed.ids[observed.current];
                if current != pending.target && Instant::now() <= pending.deadline {
                    if let Some(target) = observed.ids.iter().position(|id| *id == pending.target) {
                        done = false;
                        if pending.sent_from != Some(current) {
                            let direction = if target < observed.current { VK_LEFT } else { VK_RIGHT };
                            if chord(&[VK_LWIN, VK_CONTROL, direction]) {
                                pending.sent_from = Some(current);
                                pending.sent_at = Instant::now();
                            }
                        } else if pending.sent_at.elapsed() > Duration::from_secs(2) {
                            log("Windows desktop switch was not confirmed; stopped without guessing the current desktop.");
                            done = true;
                        }
                    }
                }
            }
            if done { controller.pending = None; }
            snapshot
        })
    }
    pub fn switching() -> bool {
        DESKTOPS.with(|c| {
            let c = c.borrow();
            c.pending.is_some() || c.settle_until.is_some_and(|until| Instant::now() < until)
        })
    }
    pub fn snapshot() -> Option<Snapshot> {
        DESKTOPS.with(|c| c.borrow().snapshot.clone())
    }
    // Public IVirtualDesktopManager is used only to move Omarchy Overlay's own windows to
    // the selected desktop, so the bar follows native keyboard/Task View changes.
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
        let Some(s) = snapshot() else {
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
