//! DWM frame colors (border, caption, text) are write-only: these decide when to write them,
//! so an app's own colors are never overwritten with the default.

/// True when a write-only attribute must be written: its value differs from what we last set
/// (`previous`, the last acceptable set) or, before any change, from the recorded original.
pub fn write_only_stale(previous: Option<&[(u32, u32)]>, attribute: u32, before: u32, value: u32) -> bool {
    let last = previous
        .and_then(|set| set.iter().find(|(a, _)| *a == attribute))
        .map_or(before, |&(_, v)| v);
    value != last
}

/// True when any change we made set this write-only attribute away from its original value.
pub fn write_only_touched(acceptable: &[Vec<(u32, u32)>], attribute: u32, before: u32) -> bool {
    acceptable
        .iter()
        .flatten()
        .any(|&(a, v)| a == attribute && v != before)
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEFAULT: u32 = 0xFFFF_FFFF;
    #[test]
    fn first_apply_with_colors_off_leaves_caption_and_text_alone() {
        assert!(!write_only_stale(None, 35, DEFAULT, DEFAULT));
        assert!(!write_only_stale(None, 36, DEFAULT, DEFAULT));
        assert!(write_only_stale(None, 34, DEFAULT, 0x00ff00));
    }
    #[test]
    fn turning_colors_off_again_puts_the_default_back() {
        let previous = [(35, 0x101010), (36, 0xeeeeee)];
        assert!(write_only_stale(Some(&previous), 35, DEFAULT, DEFAULT));
        assert!(!write_only_stale(Some(&previous), 36, DEFAULT, 0xeeeeee));
    }
    #[test]
    fn restore_skips_write_only_colors_we_never_set() {
        let acceptable = vec![vec![(34, 0x00ff00), (35, DEFAULT)]];
        assert!(write_only_touched(&acceptable, 34, DEFAULT));
        assert!(!write_only_touched(&acceptable, 35, DEFAULT));
        assert!(!write_only_touched(&acceptable, 36, DEFAULT));
    }
}
