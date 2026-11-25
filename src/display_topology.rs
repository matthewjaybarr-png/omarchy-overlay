//! Omarchy's display keys, on Windows' own display topology (what Win+P switches):
//! the laptop screen off or back on, and mirroring.
use windows_sys::Win32::Devices::Display::*;

/// The topology Windows uses now: internal, clone, extend or external; None when Windows
/// would not say.
pub(super) unsafe fn current() -> Option<i32> {
    let (mut paths, mut modes) = (0u32, 0u32);
    if GetDisplayConfigBufferSizes(QDC_DATABASE_CURRENT, &mut paths, &mut modes) != 0 {
        return None;
    }
    let mut path = vec![std::mem::zeroed::<DISPLAYCONFIG_PATH_INFO>(); paths as usize];
    let mut mode = vec![std::mem::zeroed::<DISPLAYCONFIG_MODE_INFO>(); modes as usize];
    let mut topology = 0;
    (QueryDisplayConfig(
        QDC_DATABASE_CURRENT,
        &mut paths,
        path.as_mut_ptr(),
        &mut modes,
        mode.as_mut_ptr(),
        &mut topology,
    ) == 0)
        .then_some(topology)
}
unsafe fn set(flag: u32) -> bool {
    SetDisplayConfig(0, std::ptr::null(), 0, std::ptr::null(), flag | SDC_APPLY) == 0
}
/// Omarchy's Super+Ctrl+Delete: the laptop screen off (external screens only), or back
/// on (extended). The text says what happened.
pub(super) unsafe fn toggle_laptop() -> Result<&'static str, String> {
    if current() == Some(DISPLAYCONFIG_TOPOLOGY_EXTERNAL) {
        set(SDC_TOPOLOGY_EXTEND).then_some("Laptop screen on").ok_or_else(|| "Windows did not turn the laptop screen back on.".into())
    } else {
        set(SDC_TOPOLOGY_EXTERNAL)
            .then_some("Laptop screen off")
            .ok_or_else(|| "Windows did not turn the laptop screen off: is an external screen connected?".into())
    }
}
/// Omarchy's mirror toggle: the same picture on every screen, or extended again.
pub(super) unsafe fn toggle_mirror() -> Result<&'static str, String> {
    if current() == Some(DISPLAYCONFIG_TOPOLOGY_CLONE) {
        set(SDC_TOPOLOGY_EXTEND).then_some("Screens extended").ok_or_else(|| "Windows did not extend the screens.".into())
    } else {
        set(SDC_TOPOLOGY_CLONE).then_some("Screens mirrored").ok_or_else(|| "Windows did not mirror the screens: is a second screen connected?".into())
    }
}
