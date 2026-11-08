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
pub(super) const NAMES: &[(&str, &str)] = &[
    ("neon-ridge", "Neon Ridge"),
    ("tokyo-night", "Tokyo Night"),
    ("jade", "Jade"),
    ("nord", "Nord"),
    ("paper", "Paper"),
    ("alpenglow-night", "Alpenglow Night"),
];
pub(super) fn palette(name: &str) -> Option<Palette> {
    let (background, surface, text, muted, accent, border, dark) = match name {
        "jade" => (
            0x181b22, 0x232a32, 0xcedfea, 0x889bad, 0xa6da95, 0x34433e, true,
        ),
        "tokyo-night" => (
            0x1a1b26, 0x24283b, 0xc0caf5, 0x9aa5ce, 0x7aa2f7, 0x414868, true,
        ),
        "alpenglow-night" => (
            0x181529, 0x29233d, 0xe7dcff, 0xab9fc4, 0xc995f4, 0x635078, true,
        ),
        "neon-ridge" => (
            0x0a1028, 0x1a2550, 0xd7dcf5, 0x8089bd, 0x38bdfa, 0x2a3a78, true,
        ),
        "nord" => (
            0x2e3440, 0x3b4252, 0xeceff4, 0xb5c0d0, 0x88c0d0, 0x4c566a, true,
        ),
        "paper" => (
            0xf5f2ea, 0xe8e4da, 0x252a32, 0x58606b, 0x276749, 0xc8c4b8, false,
        ),
        _ => {
            let file = crate::theme_files::find(name)?;
            (
                file.background,
                file.surface,
                file.text,
                file.muted,
                file.accent,
                file.border,
                file.dark,
            )
        }
    };
    Some(Palette {
        background: rgb(background),
        surface: rgb(surface),
        text: rgb(text),
        muted: rgb(muted),
        accent: rgb(accent),
        border: rgb(border),
        dark,
    })
}
thread_local! { static CURRENT: Cell<Palette> = Cell::new(palette("neon-ridge").unwrap()); }
pub(super) fn current() -> Palette {
    CURRENT.with(Cell::get)
}
/// The color to pick as the Windows accent. Neon Ridge uses its indigo: its bright
/// cyan accent was too bright next to the navy Omarchy Overlay windows on the user's PC.
pub(super) fn windows_accent(name: &str, palette: Palette) -> u32 {
    if name == "neon-ridge" {
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
