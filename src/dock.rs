//! The dock's state that needs no Windows calls, after Omarchy's dock
//! (shell/plugins/dock/DockModel.js): which apps it shows, which window a click
//! brings forward, where each icon sits and when the dock shows.

/// When the dock shows: "floating" (Omarchy: on floating workspaces only),
/// "always", or "off".
pub const MODES: &[(&str, &str)] = &[
    ("floating", "Dock on floating workspaces"),
    ("always", "Dock always (auto-hides)"),
    ("off", "No dock"),
];

/// A pinned app: the picker favorite it comes from and the lowercase ids that
/// claim a window for it (an AppUserModelID or a program file name).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pin {
    pub key: String,
    pub ids: Vec<String>,
}

/// A window the dock may show. `ids` are lowercase, most specific first: the
/// window's AppUserModelID if it has one, then its program file name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Window {
    pub ids: Vec<String>,
    /// On the workspace in front of the user (not parked on another one).
    pub here: bool,
    pub minimized: bool,
    pub focused: bool,
}

/// One icon in the dock: a pin (index into the pins) and the windows it owns
/// (indexes into the windows), or a running app that is not pinned.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub key: String,
    pub pin: Option<usize>,
    pub windows: Vec<usize>,
}

/// Pinned apps first, in pin order, whether or not they run; then running apps
/// that are not pinned, in the order their first window appears. A window goes to
/// the pin that shares its most specific id (an app ID before a program name),
/// else it groups with other windows by that id.
pub fn items(pins: &[Pin], windows: &[Window]) -> Vec<Item> {
    let mut items: Vec<Item> = pins
        .iter()
        .enumerate()
        .map(|(i, pin)| Item {
            key: pin.key.clone(),
            pin: Some(i),
            windows: vec![],
        })
        .collect();
    for (index, window) in windows.iter().enumerate() {
        if let Some(pin) = window
            .ids
            .iter()
            .find_map(|id| pins.iter().position(|pin| pin.ids.contains(id)))
        {
            items[pin].windows.push(index);
            continue;
        }
        let key = window
            .ids
            .first()
            .cloned()
            .unwrap_or_else(|| format!("window {index}"));
        match items.iter_mut().find(|item| item.pin.is_none() && item.key == key) {
            Some(item) => item.windows.push(index),
            None => items.push(Item {
                key,
                pin: None,
                windows: vec![index],
            }),
        }
    }
    items
}

/// Keeps running apps that are not pinned in the order they joined the dock, so icons
/// do not jump around when the focus changes. `order` remembers keys across calls.
pub fn keep_order(items: &mut [Item], order: &mut Vec<String>) {
    order.retain(|key| items.iter().any(|i| i.pin.is_none() && &i.key == key));
    for item in items.iter().filter(|i| i.pin.is_none()) {
        if !order.contains(&item.key) {
            order.push(item.key.clone());
        }
    }
    let pinned = items.iter().filter(|i| i.pin.is_some()).count();
    items[pinned..].sort_by_key(|i| order.iter().position(|k| *k == i.key));
}
/// The window a click on an item brings forward, or None to launch the app.
/// Windows on the workspace in front come first, then minimized ones, then those
/// on other workspaces; a click while one of them is focused moves on to the next.
pub fn pick(item: &Item, windows: &[Window]) -> Option<usize> {
    let rank = |w: &Window| {
        if w.minimized {
            1
        } else if w.here {
            0
        } else {
            2
        }
    };
    let mut order = item.windows.clone();
    order.sort_by_key(|&i| rank(&windows[i]));
    let focused = order.iter().position(|&i| windows[i].focused);
    match focused {
        Some(at) => Some(order[(at + 1) % order.len()]),
        None => order.first().copied(),
    }
}

/// Whether the dock shows on a monitor.
pub fn shows(mode: &str, layout: &str, full_screen: bool) -> bool {
    !full_screen
        && match mode {
            "always" => true,
            "floating" => layout == "floating",
            _ => false,
        }
}

/// Sizes in device-independent pixels.
pub const ICON: i32 = 32;
pub const SLOT: i32 = 48;
pub const PAD: i32 = 8;
pub const MARGIN: i32 = 8;

/// The dock's width and height for `count` icons, at 96 DPI.
pub fn size(count: usize) -> (i32, i32) {
    (SLOT * count.max(1) as i32 + 2 * PAD, SLOT + 2 * PAD)
}

/// Which icon an x position inside the dock falls on (dock coordinates, 96 DPI).
pub fn hit(x: i32, y: i32, count: usize) -> Option<usize> {
    let (w, h) = size(count);
    if count == 0 || x < PAD || x >= w - PAD || y < 0 || y >= h {
        return None;
    }
    Some(((x - PAD) / SLOT) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(ids: &[&str], here: bool, minimized: bool, focused: bool) -> Window {
        Window {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            here,
            minimized,
            focused,
        }
    }
    fn pin(key: &str, ids: &[&str]) -> Pin {
        Pin {
            key: key.into(),
            ids: ids.iter().map(|s| s.to_string()).collect(),
        }
    }
    #[test]
    fn pins_come_first_then_running_apps_grouped() {
        let pins = [pin("terminal", &["microsoft.windowsterminal_8wekyb3d8bbwe!app"]), pin("code", &["code.exe"])];
        let windows = [
            window(&["chrome.exe"], true, false, false),
            window(&["microsoft.windowsterminal_8wekyb3d8bbwe!app", "windowsterminal.exe"], true, false, false),
            window(&["chrome.exe"], false, false, false),
            window(&[], true, false, false),
        ];
        let items = items(&pins, &windows);
        assert_eq!(
            items.iter().map(|i| (i.key.as_str(), i.pin, i.windows.clone())).collect::<Vec<_>>(),
            vec![
                ("terminal", Some(0), vec![1]),
                ("code", Some(1), vec![]),
                ("chrome.exe", None, vec![0, 2]),
                ("window 3", None, vec![3]),
            ]
        );
    }
    #[test]
    fn running_apps_keep_their_place_when_focus_changes() {
        let mut order = vec![];
        let a = window(&["a.exe"], true, false, false);
        let b = window(&["b.exe"], true, false, false);
        let mut first = items(&[], &[a.clone(), b.clone()]);
        keep_order(&mut first, &mut order);
        // b comes to the front, so it is listed first now.
        let mut second = items(&[], &[b, a]);
        keep_order(&mut second, &mut order);
        assert_eq!(second.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), vec!["a.exe", "b.exe"]);
    }
    #[test]
    fn the_most_specific_id_picks_the_pin() {
        // Chrome and a Chrome web app share chrome.exe; the web app's own ID wins.
        let pins = [pin("chrome", &["chrome.exe"]), pin("chatgpt", &["chrome._crx_abc"])];
        let windows = [window(&["chrome._crx_abc", "chrome.exe"], true, false, false)];
        let items = items(&pins, &windows);
        assert_eq!((items[0].windows.clone(), items[1].windows.clone()), (vec![], vec![0]));
    }
    #[test]
    fn a_window_matches_a_pin_by_any_id() {
        let pins = [pin("notepad", &["notepad.exe"])];
        let windows = [window(&["microsoft.windowsnotepad_8wekyb3d8bbwe!app", "notepad.exe"], true, false, false)];
        assert_eq!(items(&pins, &windows)[0].windows, vec![0]);
    }
    #[test]
    fn clicks_prefer_this_workspace_then_minimized_then_elsewhere_and_cycle() {
        let mut windows = vec![
            window(&["a"], false, false, false),
            window(&["a"], true, true, false),
            window(&["a"], true, false, false),
        ];
        let item = Item { key: "a".into(), pin: None, windows: vec![0, 1, 2] };
        assert_eq!(pick(&item, &windows), Some(2));
        windows[2].focused = true;
        assert_eq!(pick(&item, &windows), Some(1));
        windows[2].focused = false;
        windows[1].focused = true;
        assert_eq!(pick(&item, &windows), Some(0));
        windows[1].focused = false;
        windows[0].focused = true;
        assert_eq!(pick(&item, &windows), Some(2));
        let idle = Item { key: "b".into(), pin: Some(0), windows: vec![] };
        assert_eq!(pick(&idle, &windows), None);
    }
    #[test]
    fn shows_on_floating_workspaces_or_always_but_never_over_full_screen() {
        assert!(shows("floating", "floating", false));
        assert!(!shows("floating", "dwindle", false));
        assert!(shows("always", "dwindle", false));
        assert!(!shows("always", "dwindle", true));
        assert!(!shows("off", "floating", false));
    }
    #[test]
    fn hit_finds_the_icon_under_the_pointer() {
        assert_eq!(size(3), (3 * SLOT + 2 * PAD, SLOT + 2 * PAD));
        assert_eq!(hit(PAD, 10, 3), Some(0));
        assert_eq!(hit(PAD + SLOT, 10, 3), Some(1));
        assert_eq!(hit(PAD + 3 * SLOT - 1, 10, 3), Some(2));
        assert_eq!(hit(PAD - 1, 10, 3), None);
        assert_eq!(hit(PAD + 3 * SLOT, 10, 3), None);
        assert_eq!(hit(20, 10, 0), None);
    }
}
