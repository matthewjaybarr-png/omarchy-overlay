//! Public Windows Shell discovery and Core Audio status, with scoped COM ownership.
use ::windows::{
    core::{w, Result, PWSTR},
    Win32::{
        Media::Audio::{
            eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::Shell::{
            BHID_EnumItems, IEnumShellItems, IShellItem, SHCreateItemFromParsingName,
            SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
        },
    },
};
struct Apartment(bool);
impl Apartment {
    unsafe fn enter() -> Self {
        Self(CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok())
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() }
        }
    }
}
unsafe fn endpoint() -> Result<IAudioEndpointVolume> {
    let devices: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER)?;
    devices
        .GetDefaultAudioEndpoint(eRender, eMultimedia)?
        .Activate(CLSCTX_INPROC_SERVER, None)
}
pub(super) unsafe fn volume() -> Option<(u8, bool)> {
    let _apartment = Apartment::enter();
    let endpoint = endpoint().ok()?;
    let level = endpoint.GetMasterVolumeLevelScalar().ok()?;
    let muted = endpoint.GetMute().ok()?.as_bool();
    Some(((level.clamp(0.0, 1.0) * 100.0).round() as u8, muted))
}
pub(super) unsafe fn adjust_volume(delta: i32) {
    let _apartment = Apartment::enter();
    if let Ok(endpoint) = endpoint() {
        if let Ok(level) = endpoint.GetMasterVolumeLevelScalar() {
            let _ = endpoint.SetMasterVolumeLevelScalar(
                (level + delta as f32 / 100.0).clamp(0.0, 1.0),
                std::ptr::null(),
            );
        }
    }
}
pub(super) unsafe fn toggle_mute() {
    let _apartment = Apartment::enter();
    if let Ok(endpoint) = endpoint() {
        if let Ok(muted) = endpoint.GetMute() {
            let _ = endpoint.SetMute(!muted.as_bool(), std::ptr::null());
        }
    }
}
unsafe fn take_string(value: PWSTR) -> String {
    let text = value.to_string().unwrap_or_default();
    CoTaskMemFree(Some(value.0.cast()));
    text
}
pub(super) unsafe fn applications() -> Vec<(String, String)> {
    let _apartment = Apartment::enter();
    let mut entries = Vec::new();
    let Ok(folder): Result<IShellItem> = SHCreateItemFromParsingName(w!("shell:AppsFolder"), None)
    else {
        return entries;
    };
    let Ok(items): Result<IEnumShellItems> = folder.BindToHandler(None, &BHID_EnumItems) else {
        return entries;
    };
    // Bound enumeration even if an installed shell extension misbehaves.
    for _ in 0..4096 {
        let mut next = [None];
        let mut fetched = 0;
        if items.Next(&mut next, Some(&mut fetched)).is_err() || fetched == 0 {
            break;
        }
        let Some(item) = next[0].take() else {
            break;
        };
        if let Ok(name) = item.GetDisplayName(SIGDN_NORMALDISPLAY) {
            let name = take_string(name);
            if let Ok(path) = item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING) {
                let path = take_string(path);
                if !name.is_empty() && !path.is_empty() {
                    entries.push((name, path));
                }
            }
        }
    }
    entries
}

/// Obtain an owned icon through the Shell namespace, including packaged apps.
pub(super) unsafe fn shell_icon(
    target: &str,
) -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHParseDisplayName, SHFILEINFOW, SHGFI_ICON, SHGFI_PIDL, SHGFI_SMALLICON,
    };
    let _apartment = Apartment::enter();
    let text = super::wide(target);
    let mut pidl = std::ptr::null_mut();
    let mut info: SHFILEINFOW = std::mem::zeroed();
    if SHParseDisplayName(
        text.as_ptr(),
        std::ptr::null_mut(),
        &mut pidl,
        0,
        std::ptr::null_mut(),
    ) >= 0
        && !pidl.is_null()
    {
        SHGetFileInfoW(
            pidl.cast(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON | SHGFI_PIDL,
        );
        CoTaskMemFree(Some(pidl.cast()));
    }
    if info.hIcon.is_null() {
        SHGetFileInfoW(
            text.as_ptr(),
            0,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );
    }
    info.hIcon
}

/// An owned icon `px` square, drawn by the Shell at that size (the app grid's big icons).
/// Null when the Shell has no picture for `target`.
pub(super) unsafe fn shell_picture(
    target: &str,
    px: i32,
) -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    use ::windows::{
        core::PCWSTR,
        Win32::{
            Foundation::SIZE,
            Graphics::Gdi::DeleteObject,
            UI::Shell::{IShellItemImageFactory, SIIGBF_ICONONLY},
        },
    };
    use windows_sys::Win32::{
        Graphics::Gdi::CreateBitmap,
        UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO},
    };
    let _apartment = Apartment::enter();
    let text = super::wide(target);
    let Ok(factory): Result<IShellItemImageFactory> =
        SHCreateItemFromParsingName(PCWSTR(text.as_ptr()), None)
    else {
        return std::ptr::null_mut();
    };
    let Ok(color) = factory.GetImage(SIZE { cx: px, cy: px }, SIIGBF_ICONONLY) else {
        return std::ptr::null_mut();
    };
    // The 32-bit picture carries its own alpha; the mask only has to exist.
    let bits = vec![0u8; (px as usize).div_ceil(16) * 2 * px as usize];
    let mask = CreateBitmap(px, px, 1, 1, bits.as_ptr().cast());
    let info = ICONINFO {
        fIcon: 1,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: color.0 as _,
    };
    let icon = CreateIconIndirect(&info);
    windows_sys::Win32::Graphics::Gdi::DeleteObject(mask);
    let _ = DeleteObject(color.into());
    icon
}

/// Reads one string property from a shell property store, such as a window's
/// AppUserModelID or the relaunch icon a browser sets on a web-app window.
unsafe fn store_string(
    store: &::windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore,
    key: &::windows::Win32::Foundation::PROPERTYKEY,
) -> Option<String> {
    use ::windows::Win32::System::Com::StructuredStorage::{
        PropVariantClear, PropVariantToStringAlloc,
    };
    let mut value = store.GetValue(key).ok()?;
    let text = PropVariantToStringAlloc(&value)
        .ok()
        .map(|s| take_string(s));
    let _ = PropVariantClear(&mut value);
    text.filter(|t| !t.trim().is_empty())
}
/// AppUserModelID and relaunch icon resource that a window carries, if any.
pub(super) unsafe fn window_app(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> (Option<String>, Option<String>) {
    use ::windows::Win32::{
        Foundation::HWND,
        Storage::EnhancedStorage::{PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchIconResource},
        UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
    };
    let _apartment = Apartment::enter();
    let Ok(store) = SHGetPropertyStoreForWindow::<IPropertyStore>(HWND(hwnd)) else {
        return (None, None);
    };
    (
        store_string(&store, &PKEY_AppUserModel_ID),
        store_string(&store, &PKEY_AppUserModel_RelaunchIconResource),
    )
}
/// AppUserModelID stored in a Start-menu shortcut.
pub(super) unsafe fn shortcut_app_id(path: &std::path::Path) -> Option<String> {
    use ::windows::{
        core::HSTRING,
        Win32::{
            Storage::EnhancedStorage::PKEY_AppUserModel_ID,
            UI::Shell::PropertiesSystem::{
                IPropertyStore, SHGetPropertyStoreFromParsingName, GPS_DEFAULT,
            },
        },
    };
    let _apartment = Apartment::enter();
    let store: IPropertyStore = SHGetPropertyStoreFromParsingName(
        &HSTRING::from(&*path.to_string_lossy()),
        None,
        GPS_DEFAULT,
    )
    .ok()?;
    store_string(&store, &PKEY_AppUserModel_ID)
}
/// Small icon from an "path,index" icon resource string.
pub(super) unsafe fn resource_icon(
    resource: &str,
) -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    use windows_sys::Win32::UI::Shell::{ExtractIconExW, PathParseIconLocationW};
    let mut text = super::wide(resource.trim().trim_start_matches('@'));
    text.resize(text.len().max(260), 0);
    let index = PathParseIconLocationW(text.as_mut_ptr());
    let path = String::from_utf16_lossy(&text[..text.iter().position(|&c| c == 0).unwrap_or(0)]);
    let path = path.trim_matches('"');
    let mut small = std::ptr::null_mut();
    if !path.is_empty() && std::path::Path::new(path).exists() {
        ExtractIconExW(
            super::wide(path).as_ptr(),
            index,
            std::ptr::null_mut(),
            &mut small,
            1,
        );
    }
    small
}
