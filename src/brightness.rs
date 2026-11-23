//! Brightness of external monitors over DDC/CI (VCP code 0x10), the documented
//! monitor-configuration API. Monitors that do not answer are skipped: laptop
//! panels, many docks and KVM switches, and anything in HDR mode.
use std::{mem::zeroed, ptr::null_mut};
use windows_sys::Win32::{
    Devices::Display::{
        DestroyPhysicalMonitors, GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, SetVCPFeature,
        PHYSICAL_MONITOR,
    },
    Foundation::{LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
};

/// Moves every monitor's brightness by `percent` of its range. DDC takes tens of
/// milliseconds per monitor, so it runs off the UI thread.
pub(super) fn adjust(percent: i32) {
    std::thread::spawn(move || unsafe {
        EnumDisplayMonitors(null_mut(), std::ptr::null(), Some(each), percent as isize);
    });
}
unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, percent: LPARAM) -> i32 {
    let mut count = 0;
    if GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) == 0 || count == 0 {
        return 1;
    }
    let mut physical = vec![zeroed::<PHYSICAL_MONITOR>(); count as usize];
    if GetPhysicalMonitorsFromHMONITOR(monitor, count, physical.as_mut_ptr()) != 0 {
        for p in &physical {
            let handle = p.hPhysicalMonitor;
            let (mut current, mut maximum) = (0u32, 0u32);
            if GetVCPFeatureAndVCPFeatureReply(handle, 0x10, null_mut(), &mut current, &mut maximum)
                != 0
                && maximum > 0
            {
                let next = crate::commands::brightness_step(current, maximum, percent as i32);
                if next != current {
                    SetVCPFeature(handle, 0x10, next);
                }
            }
        }
        DestroyPhysicalMonitors(count, physical.as_ptr());
    }
    1
}
