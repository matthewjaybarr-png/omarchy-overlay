//! Omarchy Overlay menu materials (0.17, request 3.4): how see-through the menu's tint layer
//! can be while its text stays readable. The menu paints its rows with GDI into a
//! 32-bit bitmap; pixels left at the background colour become a premultiplied tint
//! over the Mica or Acrylic backdrop, and everything else (text, icons, highlights)
//! stays opaque. The tint is only as see-through as WCAG contrast allows.
//!
//! Colours here are Windows COLORREF values (0x00BBGGRR), as theme.rs stores them.

/// Contrast the menu's text must keep over any window behind it (WCAG AA body text).
pub const MIN_CONTRAST: f64 = 4.5;

/// DWMWA_SYSTEMBACKDROP_TYPE value for a material name: 1 none, 2 Mica, 3 Acrylic.
pub fn backdrop_type(material: &str) -> u32 {
    match material {
        "mica" => 2,
        "acrylic" => 3,
        _ => 1,
    }
}

fn channels(c: u32) -> [f64; 3] {
    [
        (c & 255) as f64,
        ((c >> 8) & 255) as f64,
        ((c >> 16) & 255) as f64,
    ]
}

fn luminance(rgb: [f64; 3]) -> f64 {
    let lin = |v: f64| {
        let s = v / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2])
}

/// WCAG contrast ratio between two luminances.
pub fn contrast(a: f64, b: f64) -> f64 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

/// The colour seen through a tint of `alpha` (0–255) laid over `under`.
pub fn over(tint: u32, alpha: u8, under: [f64; 3]) -> [f64; 3] {
    let a = alpha as f64 / 255.0;
    let t = channels(tint);
    [
        t[0] * a + under[0] * (1.0 - a),
        t[1] * a + under[1] * (1.0 - a),
        t[2] * a + under[2] * (1.0 - a),
    ]
}

/// True when every text colour keeps `MIN_CONTRAST` over the tint with both a white
/// and a black window behind it, and stays on the same side (lighter or darker) of the
/// background. Any other colour behind sits between those two extremes channel by
/// channel, so it is covered too: the Neon Ridge wallpaper, for example.
pub fn readable(background: u32, texts: &[u32], alpha: u8) -> bool {
    let white = luminance(over(background, alpha, [255.0; 3]));
    let black = luminance(over(background, alpha, [0.0; 3]));
    texts.iter().all(|&t| {
        let l = luminance(channels(t));
        contrast(l, white) >= MIN_CONTRAST
            && contrast(l, black) >= MIN_CONTRAST
            && ((l > white) == (l > black))
    })
}

/// The most see-through tint (lowest alpha, never below 50%) that keeps every text
/// colour readable. A palette that cannot be made readable stays fully opaque.
pub fn tint_alpha(background: u32, texts: &[u32]) -> u8 {
    (128..=255u16)
        .map(|a| a as u8)
        .find(|&a| readable(background, texts, a))
        .unwrap_or(255)
}

/// Turns a GDI-painted 32-bit bitmap (BGRA in memory, alpha left at 0 by GDI) into a
/// premultiplied one: background pixels become the tint at `alpha`, all others opaque.
pub fn apply(pixels: &mut [u32], background: u32, alpha: u8) {
    // COLORREF 0x00BBGGRR -> DIB pixel 0x00RRGGBB.
    let bg = ((background & 255) << 16) | (background & 0xff00) | ((background >> 16) & 255);
    let a = alpha as u32;
    let premul = |shift: u32| (((bg >> shift) & 255) * a / 255) << shift;
    let tint = (a << 24) | premul(16) | premul(8) | premul(0);
    for p in pixels.iter_mut() {
        *p = if *p & 0x00ff_ffff == bg {
            tint
        } else {
            *p | 0xff00_0000
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn rgb(v: u32) -> u32 {
        ((v & 255) << 16) | (v & 0xff00) | ((v >> 16) & 255)
    }
    /// (background, text, muted, accent) for the built-in themes, as in theme.rs.
    const THEMES: [(&str, u32, u32, u32, u32); 6] = [
        ("neon-ridge", 0x0a1028, 0xd7dcf5, 0x8089bd, 0x38bdfa),
        ("tokyo-night", 0x1a1b26, 0xc0caf5, 0x9aa5ce, 0x7aa2f7),
        ("jade", 0x181b22, 0xcedfea, 0x889bad, 0xa6da95),
        ("nord", 0x2e3440, 0xeceff4, 0xb5c0d0, 0x88c0d0),
        ("paper", 0xf5f2ea, 0x252a32, 0x58606b, 0x276749),
        ("alpenglow-night", 0x181529, 0xe7dcff, 0xab9fc4, 0xc995f4),
    ];

    #[test]
    fn every_builtin_theme_keeps_4_5_to_1_over_white_black_and_neon_ridge() {
        // Sampled from wallpapers/neon-ridge.jpg (64x36): darkest, median, brightest
        // and most saturated pixels.
        let wallpaper = [
            [0x00, 0x00, 0x08],
            [0x2d, 0x26, 0x66],
            [0x79, 0xed, 0xfa],
            [0x0b, 0x5d, 0xba],
        ];
        for (name, bg, text, muted, accent) in THEMES {
            let texts = [rgb(text), rgb(muted), rgb(accent)];
            let alpha = tint_alpha(rgb(bg), &texts);
            assert!(readable(rgb(bg), &texts, alpha), "{name}");
            let unders = [[255.0; 3], [0.0; 3]]
                .into_iter()
                .chain(wallpaper.map(|c: [i32; 3]| c.map(|v| v as f64)));
            for under in unders {
                let seen = luminance(over(rgb(bg), alpha, under));
                for t in texts {
                    let c = contrast(luminance(channels(t)), seen);
                    assert!(
                        c >= MIN_CONTRAST,
                        "{name} alpha {alpha} under {under:?}: {c:.2}"
                    );
                }
            }
        }
    }

    #[test]
    fn neon_ridge_tint_is_actually_see_through() {
        let alpha = tint_alpha(
            rgb(0x0a1028),
            &[rgb(0xd7dcf5), rgb(0x8089bd), rgb(0x38bdfa)],
        );
        assert!(
            alpha < 255,
            "Neon Ridge should let the material show: {alpha}"
        );
    }

    #[test]
    fn unreadable_palette_stays_opaque() {
        assert_eq!(tint_alpha(rgb(0x808080), &[rgb(0x858585)]), 255);
    }

    #[test]
    fn apply_premultiplies_background_and_keeps_text_opaque() {
        let bg = rgb(0x0a1028); // COLORREF
        let mut px = [0x000a_1028, 0x00d7_dcf5];
        apply(&mut px, bg, 128);
        assert_eq!(px[0] >> 24, 128);
        assert_eq!((px[0] >> 16) & 255, 0x0a * 128 / 255);
        assert_eq!(px[0] & 255, 0x28 * 128 / 255);
        assert_eq!(px[1], 0xffd7_dcf5);
    }

    #[test]
    fn backdrop_types() {
        assert_eq!(backdrop_type("solid"), 1);
        assert_eq!(backdrop_type("mica"), 2);
        assert_eq!(backdrop_type("acrylic"), 3);
    }
}
