//! New defaults reach existing config files: a three-way merge of the user's binds.conf
//! and menu.json with the new built-in versions, against the previous version's example
//! (config\examples, written by the last version that ran). The user's choices win:
//! - a key or menu entry that is new in this version is added;
//! - a line the user never changed is updated to the new default;
//! - anything the user changed or deleted stays as it is.

use crate::binds;

/// 0.6.0's built-in files: the merge base for a folder that has no record yet.
pub const PREVIOUS_BINDS: &str = include_str!("previous_defaults/binds.conf");
pub const PREVIOUS_MENU: &str = include_str!("previous_defaults/menu.json");

/// The version of a file the user's copy last took defaults from. `record` is what the
/// last version wrote outside the shipped files (config\.defaults). Without it,
/// config\examples is used unless it already holds this version's text: unzipping a new
/// release over the old folder replaces it, so it no longer shows what the user had.
/// Then the last release before the record existed is the best guess.
pub fn base(record: Option<String>, examples: Option<String>, current: &str, previous: &str) -> String {
    let normal = |t: String| t.replace("\r\n", "\n");
    if let Some(record) = record {
        return normal(record);
    }
    match examples.map(normal) {
        Some(examples) if examples != current => examples,
        _ => previous.replace("\r\n", "\n"),
    }
}

/// The key a binds.conf line binds, or None for comments and lines that don't parse.
fn bind_key(line: &str) -> Option<binds::Key> {
    binds::parse(line).ok()?.first().map(|b| b.key)
}
fn bind_lines(text: &str) -> Vec<(binds::Key, String)> {
    text.lines()
        .filter_map(|l| bind_key(l).map(|k| (k, l.trim().to_string())))
        .collect()
}
/// The merged binds.conf, or None when nothing changes. `base` is the previous
/// version's built-in binds.conf, empty when unknown.
pub fn merge_binds(base: &str, user: &str, new: &str, version: &str) -> Option<String> {
    let base = bind_lines(base);
    let ours = bind_lines(user);
    let mut lines: Vec<String> = user.lines().map(str::to_string).collect();
    let mut added = vec![];
    let mut changed = false;
    for (key, text) in bind_lines(new) {
        let before = base.iter().find(|(k, _)| *k == key).map(|(_, t)| t.as_str());
        match ours.iter().find(|(k, _)| *k == key) {
            Some((_, mine)) => {
                // Untouched since the last default, and the default moved on: follow it.
                if Some(mine.as_str()) == before && *mine != text {
                    if let Some(line) = lines.iter_mut().find(|l| l.trim() == mine) {
                        *line = text.clone();
                        changed = true;
                    }
                }
            }
            // Deleted by the user (the last default had it): leave it out.
            None if before.is_some() => {}
            None => added.push(text),
        }
    }
    if !added.is_empty() {
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        lines.push(String::new());
        lines.push(format!("# Added by Omarchy Overlay {version}:"));
        lines.extend(added);
        changed = true;
    }
    changed.then(|| lines.join("\n") + "\n")
}

type Json = serde_json::Value;
fn menus(v: &Json) -> Vec<Json> {
    v["menus"].as_array().cloned().unwrap_or_default()
}
fn menu_by_id<'a>(list: &'a [Json], id: &str) -> Option<&'a Json> {
    list.iter().find(|m| m["id"].as_str().is_some_and(|i| i.eq_ignore_ascii_case(id)))
}
fn has_command(menu: &Json, command: &str) -> bool {
    menu["items"].as_array().is_some_and(|items| items.iter().any(|i| i["command"] == command))
}
/// The merged menu.json, or None when nothing changes or a file doesn't parse.
pub fn merge_menus(base: &str, user: &str, new: &str) -> Option<String> {
    let base: Json = serde_json::from_str(base).unwrap_or(Json::Null);
    let mut ours: Json = serde_json::from_str(user).ok()?;
    let new: Json = serde_json::from_str(new).ok()?;
    let base_menus = menus(&base);
    let mut changed = false;
    for menu in menus(&new) {
        let id = menu["id"].as_str().unwrap_or_default().to_string();
        let before = menu_by_id(&base_menus, &id);
        let list = ours["menus"].as_array_mut()?;
        let Some(mine) = list.iter_mut().find(|m| m["id"].as_str().is_some_and(|i| i.eq_ignore_ascii_case(&id))) else {
            if before.is_none() {
                list.push(menu.clone());
                changed = true;
            }
            continue;
        };
        let new_items = menu["items"].as_array().cloned().unwrap_or_default();
        for (index, item) in new_items.iter().enumerate() {
            let command = item["command"].as_str().unwrap_or_default();
            if has_command(mine, command) || before.is_some_and(|b| has_command(b, command)) {
                continue;
            }
            let items = mine["items"].as_array_mut()?;
            // After the item that comes before it in the new default, if the user has it.
            let at = new_items[..index]
                .iter()
                .rev()
                .find_map(|prev| items.iter().position(|i| i["command"] == prev["command"]))
                .map_or(items.len(), |p| p + 1);
            items.insert(at, item.clone());
            changed = true;
        }
    }
    changed.then(|| format_menus(&ours))
}
/// menu.json written like the example: one line per item.
fn format_menus(v: &Json) -> String {
    let mut out = String::from("{\n  \"menus\": [\n");
    let list = menus(v);
    for (m, menu) in list.iter().enumerate() {
        out.push_str("    {\n");
        if let Some(fields) = menu.as_object() {
            for (key, value) in fields.iter().filter(|(k, _)| k.as_str() != "items") {
                out.push_str(&format!("      {}: {},\n", Json::String(key.clone()), value));
            }
        }
        out.push_str("      \"items\": [\n");
        let items = menu["items"].as_array().cloned().unwrap_or_default();
        for (i, item) in items.iter().enumerate() {
            let inner: Vec<String> = item
                .as_object()
                .map(|o| o.iter().map(|(k, v)| format!("{}: {}", Json::String(k.clone()), v)).collect())
                .unwrap_or_default();
            out.push_str(&format!("        {{ {} }}{}\n", inner.join(", "), if i + 1 < items.len() { "," } else { "" }));
        }
        out.push_str("      ]\n");
        out.push_str(&format!("    }}{}\n", if m + 1 < list.len() { "," } else { "" }));
    }
    out.push_str("  ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_merge_base_survives_unzipping_a_release_over_the_folder() {
        let (old, new) = ("old\n", "new\n");
        // The record wins, whatever the examples say.
        assert_eq!(base(Some("rec\r\n".into()), Some(new.into()), new, old), "rec\n");
        // No record: examples the last version wrote are still the base.
        assert_eq!(base(None, Some("mine\n".into()), new, old), "mine\n");
        // Examples replaced by the new zip, or missing: the previous release.
        assert_eq!(base(None, Some("new\r\n".into()), new, old), old);
        assert_eq!(base(None, None, new, old), old);
        // 0.6.0's Toggle menu lacks the gaps row, so it is added, not taken as deleted.
        let user = PREVIOUS_MENU.replace("\r\n", "\n");
        let merged = merge_menus(&base(None, Some(crate::menu_model::DEFAULT.into()), crate::menu_model::DEFAULT, PREVIOUS_MENU), &user, crate::menu_model::DEFAULT).unwrap();
        assert!(merged.contains("toggle-gaps"));
    }
    const BASE: &str = "# keys\nSuper+W, Close window, action close-window\nSuper+Shift+Backspace, Toggle window gaps, page gaps\nSuper+M, Minimize window, minimize-window\n";
    const NEW: &str = "# keys\nSuper+W, Close window, action close-window\nSuper+Shift+Backspace, Toggle window gaps, toggle-gaps\nSuper+M, Minimize window, minimize-window\nSuper+Print, Pick a color from the screen, color-picker\n";
    #[test]
    fn binds_gain_new_keys_and_follow_untouched_defaults() {
        let merged = merge_binds(BASE, BASE, NEW, "0.6.0").unwrap();
        assert!(merged.contains("Super+Shift+Backspace, Toggle window gaps, toggle-gaps"));
        assert!(merged.contains("# Added by Omarchy Overlay 0.6.0:\nSuper+Print, Pick a color from the screen, color-picker"));
        crate::binds::parse(&merged).unwrap();
        assert_eq!(merge_binds(NEW, &merged, NEW, "0.6.0"), None, "a second run changes nothing");
    }
    #[test]
    fn binds_keep_what_the_user_changed_or_deleted() {
        let user = "Super+W, Close window, action close-window\nSuper+Shift+Backspace, Opacity, action restore-all-window-opacity\n";
        let merged = merge_binds(BASE, user, NEW, "0.6.0").unwrap();
        assert!(merged.contains("Super+Shift+Backspace, Opacity, action restore-all-window-opacity"));
        assert!(!merged.contains("Super+M"), "Super+M was in the last default, so the user deleted it");
        assert!(merged.contains("Super+Print"));
        // Without a previous version, only missing keys are added.
        let merged = merge_binds("", user, NEW, "0.6.0").unwrap();
        assert!(merged.contains("Super+M, Minimize window") && merged.contains("Opacity"));
    }
    #[test]
    fn menus_gain_new_entries_in_place_and_keep_removed_ones_out() {
        let base = r#"{"menus":[{"id":"style","title":"Appearance","items":[
            {"label":"Theme","command":"page theme"},{"label":"Font","command":"page font"},
            {"label":"Old","command":"page old"}]}]}"#;
        let new = r#"{"menus":[{"id":"style","title":"Appearance","items":[
            {"label":"Theme","command":"page theme"},{"label":"VS Code colors","command":"page vscode-colors"},
            {"label":"Font","command":"page font"},{"label":"Old","command":"page old"}]},
            {"id":"extra","title":"Extra","items":[{"label":"X","command":"apps"}]}]}"#;
        // The user removed "Font" and renamed "Theme".
        let user = r#"{"menus":[{"id":"style","title":"Looks","items":[
            {"label":"Colours","command":"page theme"},{"label":"Old","command":"page old"}]}]}"#;
        let merged = merge_menus(base, user, new).unwrap();
        let v: Json = serde_json::from_str(&merged).unwrap();
        let style = &v["menus"][0];
        assert_eq!(style["title"], "Looks");
        let commands: Vec<&str> = style["items"].as_array().unwrap().iter().map(|i| i["command"].as_str().unwrap()).collect();
        assert_eq!(commands, vec!["page theme", "page vscode-colors", "page old"]);
        assert_eq!(style["items"][0]["label"], "Colours");
        assert_eq!(v["menus"][1]["id"], "extra");
        assert_eq!(merge_menus(new, &merged, new), None);
    }
    #[test]
    fn the_shipped_menu_merges_into_itself_as_valid_menus() {
        let new = crate::menu_model::DEFAULT;
        assert_eq!(merge_menus(new, new, new), None);
        let merged = merge_menus("", r#"{"menus":[{"id":"main","title":"Go","items":[{"label":"Apps","icon":"F003B","command":"apps"}]}]}"#, new).unwrap();
        crate::menu_model::parse(&merged).unwrap();
    }
}
