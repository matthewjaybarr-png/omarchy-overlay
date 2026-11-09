use std::cell::Cell;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub background: u32,
    pub surface: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub border: u32,
    pub dark: bool,
}
const fn rgb(value: u32) -> u32 {
    ((value & 255) << 16) | (value & 0xff00) | ((value >> 16) & 255)
}
pub(super) use crate::omarchy_themes::NAMES;
/// A theme by id: a user theme first, then an embedded one.
pub(super) fn palette(name: &str) -> Option<Palette> {
    let t = crate::theme_files::resolve(name)?;
    Some(Palette {
        background: rgb(t.background),
        surface: rgb(t.surface),
        text: rgb(t.text),
        muted: rgb(t.muted),
        accent: rgb(t.accent),
        border: rgb(t.border),
        dark: t.dark,
    })
}
thread_local! { static CURRENT: Cell<Palette> = Cell::new(palette("neon-ridge").unwrap()); }
pub(super) fn current() -> Palette {
    CURRENT.with(Cell::get)
}
/// The color to pick as the Windows accent. Neon Ridge uses its indigo: its bright
/// cyan accent was too bright next to the navy Omarchy Overlay windows on the user's PC.
pub(super) fn windows_accent(name: &str, palette: Palette) -> u32 {
    // A removed theme falls back to Neon Ridge, so it takes Neon Ridge's accent too.
    if name == "neon-ridge" || crate::theme_files::resolve(name).is_none() {
        palette.border
    } else {
        palette.accent
    }
}
/// Sets the Omarchy Overlay palette; an unknown name (a removed theme file) falls back to Neon Ridge.
pub(super) fn set(name: &str) -> bool {
    let found = palette(name);
    CURRENT.with(|cell| cell.set(found.unwrap_or_else(|| palette("neon-ridge").unwrap())));
    found.is_some()
}

