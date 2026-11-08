//! Menu fonts (0.17, K1/K2). The bundled JetBrains Mono Nerd Fonts in fonts\ are loaded
//! for this process only (FR_PRIVATE keeps them out of the Windows font list). When
//! fonts\ is missing, a JetBrains Mono Nerd Font installed in Windows is used instead,
//! and only when neither exists do menus fall back to Segoe UI with plain letters.
use super::*;
use crate::font_names::{self, Source};
use std::{cell::Cell, collections::BTreeSet};

pub(super) const ICON_FAMILY: &str = font_names::ICON_FAMILY;
const FILES: [&str; 4] = [
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Bold.ttf",
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNerdFont-Bold.ttf",
];
thread_local! {
    static LOADED: RefCell<Vec<Vec<u16>>> = const { RefCell::new(vec![]) };
    static FAMILIES: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
    static SOURCE: Cell<Source> = const { Cell::new(Source::None) };
    static WARNED: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}
fn folder() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("fonts"))
}
/// Every family GDI can draw, bundled ones included (EnumFontFamiliesEx).
unsafe fn refresh() {
    let names = ui_choices::fonts();
    FAMILIES.with(|f| *f.borrow_mut() = names.into_iter().collect());
}
fn has(name: &str) -> bool {
    FAMILIES.with(|f| f.borrow().contains(name))
}
/// Loads fonts\ beside OmarchyOverlay.exe, or finds an installed Nerd Font, and logs which.
/// Returns false when menus must use plain letters instead of icons.
pub(super) unsafe fn load() -> bool {
    let folder = folder();
    LOADED.with(|loaded| {
        let mut loaded = loaded.borrow_mut();
        for file in FILES {
            let Some(path) = folder.as_ref().map(|f| f.join(file)) else {
                continue;
            };
            if !path.exists() {
                continue;
            }
            let path = wide(&path.to_string_lossy());
            if AddFontResourceExW(path.as_ptr(), FR_PRIVATE, null()) > 0 {
                loaded.push(path);
            }
        }
    });
    refresh();
    let bundled = LOADED.with(|l| !l.borrow().is_empty());
    let icon = font_names::icon_face(&has);
    let source = match (bundled, icon) {
        (true, Some(_)) => Source::Bundled,
        (false, Some(_)) => Source::Installed,
        _ => Source::None,
    };
    SOURCE.with(|s| s.set(source));
    let place = folder.map_or_else(|| "fonts".into(), |f| f.display().to_string());
    log(&match source {
        Source::Bundled => format!("Menu fonts: {} ({place}).", source.label()),
        Source::Installed => format!(
            "Menu fonts: {} (fonts folder missing at {place}; using the Windows font {}).",
            source.label(),
            icon.unwrap_or_default()
        ),
        Source::None => format!(
            "Menu fonts: {} (no fonts folder at {place} and no JetBrains Mono Nerd Font installed); menus use Segoe UI with plain letters.",
            source.label()
        ),
    });
    source != Source::None
}
/// True when the Nerd Font icons can be drawn.
pub(super) fn available() -> bool {
    SOURCE.with(Cell::get) != Source::None
}
pub(super) fn source() -> Source {
    SOURCE.with(Cell::get)
}
/// Families for the menu font list: JetBrains Mono NL first, then what Windows has.
pub(super) fn choices() -> Vec<String> {
    let installed: Vec<String> = FAMILIES.with(|f| f.borrow().iter().cloned().collect());
    font_names::menu_choices(&installed)
}
/// True when this family can be drawn as chosen (not the Segoe UI fallback).
pub(super) fn exists(family: &str) -> bool {
    !font_names::text_face(family, &has).1
}
pub(super) unsafe fn unload() {
    LOADED.with(|loaded| {
        for path in loaded.borrow_mut().drain(..) {
            RemoveFontResourceExW(path.as_ptr(), FR_PRIVATE, null());
        }
    });
}
/// The GDI name to draw `family` with. Unknown fonts fall back to Segoe UI and are
/// logged once; the profile keeps the name, so it works again if the font comes back.
fn face(family: &str) -> String {
    let (face, missing) = font_names::text_face(family, &has);
    if missing && WARNED.with(|w| w.borrow_mut().insert(family.to_string())) {
        log(&format!(
            "Font \"{family}\" is not available; Segoe UI is used instead."
        ));
    }
    face
}
unsafe fn make(face: &str, points: i32, bold: bool, dpi: u32) -> HFONT {
    CreateFontW(
        -(points * dpi.max(96) as i32 / 72),
        0,
        0,
        0,
        if bold { FW_BOLD } else { FW_NORMAL } as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        DEFAULT_PITCH as u32,
        wide(face).as_ptr(),
    )
}
/// A text font of the given family scaled for the window's DPI.
pub(super) unsafe fn create(family: &str, points: i32, bold: bool, dpi: u32) -> HFONT {
    make(&face(family), points, bold, dpi)
}
/// The icon font: always the Nerd Font, whatever the text font is.
pub(super) unsafe fn create_icon(points: i32, dpi: u32) -> HFONT {
    make(
        font_names::icon_face(&has).unwrap_or(font_names::FALLBACK),
        points,
        false,
        dpi,
    )
}
