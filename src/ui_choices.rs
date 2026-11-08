//! Native dropdown helpers; arbitrary text is reserved for names and advanced rules.
use super::*;
pub(super) unsafe fn create(
    parent: HWND,
    id: usize,
    x: i32,
    y: i32,
    width: i32,
    items: &[String],
) -> HWND {
    let h = child(
        parent,
        "COMBOBOX",
        "",
        CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        id,
        x,
        y,
        width,
        260,
    );
    for item in items {
        SendMessageW(h, CB_ADDSTRING, 0, wide(item).as_ptr() as isize);
    }
    h
}
pub(super) unsafe fn range(
    parent: HWND,
    id: usize,
    x: i32,
    y: i32,
    width: i32,
    min: i32,
    max: i32,
) -> HWND {
    create(
        parent,
        id,
        x,
        y,
        width,
        &(min..=max).map(|n| n.to_string()).collect::<Vec<_>>(),
    )
}
pub(super) unsafe fn select(parent: HWND, id: i32, value: &str) {
    let h = GetDlgItem(parent, id);
    let mut index = SendMessageW(
        h,
        CB_FINDSTRINGEXACT,
        usize::MAX,
        wide(value).as_ptr() as isize,
    );
    // Keep existing profile choices even when a font or value is absent from this PC's list.
    if index < 0 {
        index = SendMessageW(h, CB_ADDSTRING, 0, wide(value).as_ptr() as isize);
    }
    SendMessageW(h, CB_SETCURSEL, index as usize, 0);
}
pub(super) unsafe fn fonts() -> Vec<String> {
    unsafe extern "system" fn collect(
        font: *const LOGFONTW,
        _: *const TEXTMETRICW,
        _: u32,
        data: LPARAM,
    ) -> i32 {
        let lf = &*font;
        let end = lf
            .lfFaceName
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(lf.lfFaceName.len());
        let name = String::from_utf16_lossy(&lf.lfFaceName[..end]);
        if !name.is_empty() && !name.starts_with('@') {
            (*(data as *mut std::collections::BTreeSet<String>)).insert(name);
        }
        1
    }
    let mut names = std::collections::BTreeSet::new();
    let dc = GetDC(null_mut());
    let mut lf: LOGFONTW = zeroed();
    lf.lfCharSet = DEFAULT_CHARSET;
    if !dc.is_null() {
        EnumFontFamiliesExW(dc, &lf, Some(collect), &mut names as *mut _ as isize, 0);
        ReleaseDC(null_mut(), dc);
    }
    names.into_iter().collect()
}
pub(super) unsafe fn items(parent: HWND, id: i32) -> Vec<String> {
    let h = GetDlgItem(parent, id);
    let count = SendMessageW(h, CB_GETCOUNT, 0, 0);
    if !(0..=64).contains(&count) {
        return vec![];
    }
    (0..count)
        .filter_map(|i| {
            let len = SendMessageW(h, CB_GETLBTEXTLEN, i as usize, 0);
            if !(0..=1024).contains(&len) {
                return None;
            }
            let mut value = vec![0u16; len as usize + 1];
            SendMessageW(h, CB_GETLBTEXT, i as usize, value.as_mut_ptr() as isize);
            Some(String::from_utf16_lossy(&value[..len as usize]))
        })
        .collect()
}
