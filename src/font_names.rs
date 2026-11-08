//! Which Windows (GDI) family a menu font name maps to (0.17, K1/K2). GDI reads the
//! legacy name table, so the Nerd Font files answer to "JetBrainsMono NF" and
//! "JetBrainsMonoNL NF", not the longer names people see. Pure, so it is tested here;
//! fonts.rs feeds it the families Windows reports.

/// Where the menu's fonts come from, written to omarchy-overlay.log at start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// The fonts\ folder beside OmarchyOverlay.exe, loaded for this process only.
    Bundled,
    /// A Nerd Font installed in Windows.
    Installed,
    /// Neither: Segoe UI with plain letters instead of icons.
    None,
}
impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Bundled => "bundled",
            Source::Installed => "installed",
            Source::None => "none",
        }
    }
}

/// The name the profile uses for icons (kept for older profiles).
pub const ICON_FAMILY: &str = "JetBrainsMono Nerd Font";
/// The default menu text font since 0.17.
pub const NL_FAMILY: &str = "JetBrains Mono NL";
pub const FALLBACK: &str = "Segoe UI";

/// GDI names that carry the Nerd Font icons, best first.
pub const ICON_FACES: [&str; 4] = [
    "JetBrainsMono NF",
    "JetBrainsMonoNL NF",
    "JetBrainsMono Nerd Font Mono",
    "JetBrainsMono Nerd Font",
];

/// The GDI family for the icons, or None when no Nerd Font exists.
pub fn icon_face(has: &dyn Fn(&str) -> bool) -> Option<&'static str> {
    ICON_FACES.into_iter().find(|f| has(f))
}

/// The GDI family to draw menu text with, and whether the chosen name was missing
/// (the caller warns once; the profile stays valid).
pub fn text_face(family: &str, has: &dyn Fn(&str) -> bool) -> (String, bool) {
    let candidates: &[&str] = match family {
        NL_FAMILY => &[
            "JetBrainsMonoNL NF",
            "JetBrains Mono NL",
            "JetBrainsMono NF",
        ],
        ICON_FAMILY => &[
            "JetBrainsMono NF",
            "JetBrainsMono Nerd Font",
            "JetBrainsMonoNL NF",
        ],
        "JetBrainsMonoNL Nerd Font" => &["JetBrainsMonoNL NF", "JetBrainsMono NF"],
        other => &[other],
    };
    match candidates.iter().find(|f| has(f)) {
        Some(found) => (found.to_string(), false),
        None => (FALLBACK.into(), true),
    }
}

/// The menu font list: JetBrains Mono NL and the Nerd Font first, then the other
/// families Windows has, without GDI's vertical "@" names and Nerd Font aliases.
pub fn menu_choices(installed: &[String]) -> Vec<String> {
    let mut out = vec![NL_FAMILY.to_string(), ICON_FAMILY.to_string()];
    let hidden = |n: &str| {
        n.starts_with('@')
            || ICON_FACES.contains(&n)
            || n == "JetBrainsMonoNL NF"
            || n == "JetBrainsMonoNL Nerd Font"
            || n == NL_FAMILY
            || n == ICON_FAMILY
    };
    for name in installed {
        if !hidden(name) && !out.contains(name) {
            out.push(name.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &'static [&'static str]) -> impl Fn(&str) -> bool {
        move |n| names.contains(&n)
    }

    #[test]
    fn bundled_or_installed_nerd_font_is_found_by_its_gdi_name() {
        let both = set(&["JetBrainsMono NF", "JetBrainsMonoNL NF", "Segoe UI"]);
        assert_eq!(icon_face(&both), Some("JetBrainsMono NF"));
        assert_eq!(
            text_face(NL_FAMILY, &both),
            ("JetBrainsMonoNL NF".into(), false)
        );
        assert_eq!(
            text_face(ICON_FAMILY, &both),
            ("JetBrainsMono NF".into(), false)
        );
        let nl_only = set(&["JetBrainsMonoNL NF"]);
        assert_eq!(icon_face(&nl_only), Some("JetBrainsMonoNL NF"));
        assert_eq!(
            text_face(ICON_FAMILY, &nl_only),
            ("JetBrainsMonoNL NF".into(), false)
        );
    }

    #[test]
    fn missing_fonts_fall_back_to_segoe_ui_and_say_so() {
        let none = set(&["Segoe UI", "Consolas"]);
        assert_eq!(icon_face(&none), None);
        assert_eq!(text_face(NL_FAMILY, &none), ("Segoe UI".into(), true));
        assert_eq!(text_face("Fira Code", &none), ("Segoe UI".into(), true));
        assert_eq!(text_face("Consolas", &none), ("Consolas".into(), false));
    }

    #[test]
    fn plain_jetbrains_mono_nl_is_used_without_the_nerd_font() {
        let plain = set(&["JetBrains Mono NL"]);
        assert_eq!(
            text_face(NL_FAMILY, &plain),
            ("JetBrains Mono NL".into(), false)
        );
    }

    #[test]
    fn menu_list_puts_jetbrains_first_and_hides_aliases() {
        let installed: Vec<String> = [
            "@Malgun",
            "Arial",
            "JetBrainsMono NF",
            "JetBrainsMonoNL NF",
            "Consolas",
            "Arial",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            menu_choices(&installed),
            vec![NL_FAMILY, ICON_FAMILY, "Arial", "Consolas"]
        );
    }

    #[test]
    fn source_labels_match_the_log_wording() {
        assert_eq!(
            [Source::Bundled, Source::Installed, Source::None].map(Source::label),
            ["bundled", "installed", "none"]
        );
    }
}
