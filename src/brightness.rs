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
/// milliseconds per monitor, so it runs off the UI thread. The first monitor's new level
/// goes to `owner` for the on-screen message (OSD_MESSAGE, 2000 + percent).
pub(super) fn adjust(percent: i32, owner: windows_sys::Win32::Foundation::HWND) {
    let owner = owner as isize;
    std::thread::spawn(move || unsafe {
        let mut job = (percent, None::<u32>);
        EnumDisplayMonitors(null_mut(), std::ptr::null(), Some(each), &mut job as *mut _ as isize);
        if let Some(level) = job.1 {
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                owner as windows_sys::Win32::Foundation::HWND,
                super::OSD_MESSAGE,
                2000 + level.min(100) as usize,
                0,
            );
        }
    });
}
unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, job: LPARAM) -> i32 {
    let job = &mut *(job as *mut (i32, Option<u32>));
    let percent = job.0;
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
                let next = crate::commands::brightness_step(current, maximum, percent);
                if next != current {
                    SetVCPFeature(handle, 0x10, next);
                }
                job.1.get_or_insert(next * 100 / maximum);
            }
        }
        DestroyPhysicalMonitors(count, physical.as_ptr());
    }
    1
}
