//! Which Windows visual style each native control gets (0.21).

/// The visual style for one control: `Some("")` turns visual styles off for it,
/// `Some(name)` picks a named Windows style, `None` keeps the default theme.
pub fn dark_style(class: &str, style: u32, dark: bool) -> Option<&'static str> {
    if !dark {
        return None;
    }
    let class = class.to_ascii_lowercase();
    match class.as_str() {
        // A multi-line edit has its own scroll bar, which only the Explorer style darkens.
        "edit" if style & 0x4 != 0 => Some("DarkMode_Explorer"),
        "combobox" | "edit" => Some("DarkMode_CFD"),
        "listbox" | "scrollbar" | "msctls_trackbar32" => Some("DarkMode_Explorer"),
        "button" => match style & 0xf {
            // Push buttons are owner-drawn; the rest are check boxes, radio buttons
            // and group boxes, whose themed label would be black on navy.
            0x0 | 0x1 | 0xb => None,
            _ => Some(""),
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::dark_style;

    #[test]
    fn light_palette_keeps_the_default_theme() {
        for class in ["ComboBox", "Edit", "ListBox", "Button", "Static"] {
            assert_eq!(dark_style(class, 0x3, false), None);
        }
    }

    #[test]
    fn dark_palette_uses_windows_dark_styles() {
        assert_eq!(dark_style("ComboBox", 0, true), Some("DarkMode_CFD"));
        assert_eq!(dark_style("Edit", 0, true), Some("DarkMode_CFD"));
        assert_eq!(dark_style("Edit", 0x4, true), Some("DarkMode_Explorer")); // ES_MULTILINE
        assert_eq!(dark_style("ListBox", 0, true), Some("DarkMode_Explorer"));
        assert_eq!(dark_style("Static", 0, true), None);
    }

    #[test]
    fn dark_check_boxes_stay_unthemed_and_owner_drawn_buttons_untouched() {
        assert_eq!(dark_style("Button", 0x3, true), Some("")); // BS_AUTOCHECKBOX
        assert_eq!(dark_style("Button", 0x9, true), Some("")); // BS_AUTORADIOBUTTON
        assert_eq!(dark_style("Button", 0x7, true), Some("")); // BS_GROUPBOX
        assert_eq!(dark_style("Button", 0xb, true), None); // BS_OWNERDRAW
        assert_eq!(dark_style("Button", 0x0, true), None); // BS_PUSHBUTTON
    }
}
