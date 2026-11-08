//! Ranked app search and PC-local favorites/recent launches.
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BTreeMap};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    counter: u64,
    items: BTreeMap<String, Preference>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Preference {
    favorite: bool,
    last_used: u64,
    uses: u64,
    /// Removed from the picker with Remove → App from the picker.
    hidden: bool,
}
impl History {
    pub fn favorite(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|item| item.favorite)
    }
    pub fn toggle(&mut self, key: &str) {
        let item = self.items.entry(key.into()).or_default();
        item.favorite = !item.favorite;
    }
    pub fn launched(&mut self, key: &str) {
        self.counter = self.counter.saturating_add(1);
        let item = self.items.entry(key.into()).or_default();
        item.last_used = self.counter;
        item.uses = item.uses.saturating_add(1);
    }
    pub fn hidden(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|item| item.hidden)
    }
    pub fn set_hidden(&mut self, key: &str, hidden: bool) {
        self.items.entry(key.into()).or_default().hidden = hidden;
    }
    pub fn recent(&self, key: &str) -> bool {
        self.items.get(key).is_some_and(|item| item.last_used > 0)
    }
    pub fn rank(&self, query: &str, apps: &[(String, String)]) -> Vec<usize> {
        let query = query.to_lowercase();
        let mut matches = apps
            .iter()
            .enumerate()
            .filter(|(_, (_, key))| !self.hidden(key))
            .filter_map(|(index, (label, key))| {
                let label = label.to_lowercase();
                score(&query, &label).map(|score| {
                    let preference = self.items.get(key);
                    (
                        index,
                        (
                            score,
                            !self.favorite(key),
                            Reverse(preference.map_or(0, |item| item.uses)),
                            Reverse(preference.map_or(0, |item| item.last_used)),
                            label,
                        ),
                    )
                })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|a, b| a.1.cmp(&b.1));
        matches
            .into_iter()
            .map(|(index, _)| index)
            .take(100)
            .collect()
    }
}
fn score(query: &str, label: &str) -> Option<usize> {
    let words = query.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return Some(0);
    }
    let normalized = words.join(" ");
    if label == normalized {
        return Some(0);
    }
    if label.starts_with(&normalized) {
        return Some(10);
    }
    if words.iter().all(|word| {
        label
            .split(|c: char| !c.is_alphanumeric())
            .any(|part| part.starts_with(word))
    }) {
        return Some(20);
    }
    if words.iter().all(|word| label.contains(word)) {
        return Some(40);
    }
    // Ordered fuzzy matching supports abbreviations such as "vsc" and "wterm".
    let needle = words.join("");
    let mut wanted = needle.chars();
    let mut next = wanted.next();
    let mut span = 0;
    let mut started = false;
    for ch in label.chars() {
        if Some(ch) == next {
            next = wanted.next();
            started = true;
        }
        if started {
            span += 1;
        }
        if next.is_none() {
            return Some(80 + span - needle.chars().count());
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    fn apps() -> Vec<(String, String)> {
        [
            "Visual Studio Code",
            "Notepad++",
            "Windows Terminal",
            "Notepad",
        ]
        .into_iter()
        .map(|s| (s.into(), s.into()))
        .collect()
    }
    #[test]
    fn exact_names_beat_prefixes_and_favorites() {
        let mut history = History::default();
        history.toggle("Notepad++");
        assert_eq!(history.rank("NOTEPAD", &apps()), vec![3, 1]);
    }
    #[test]
    fn abbreviations_words_and_unicode_are_searchable() {
        let h = History::default();
        assert_eq!(h.rank("vsc", &apps()), vec![0]);
        assert_eq!(h.rank("term win", &apps()), vec![2]);
        assert_eq!(h.rank("wterm", &apps()), vec![2]);
        assert!(h.rank("不存在", &apps()).is_empty());
        assert_eq!(score("é", "éditeur"), Some(10));
    }
    #[test]
    fn empty_search_favors_pins_then_recency() {
        let mut h = History::default();
        h.launched("Windows Terminal");
        assert!(h.recent("Windows Terminal"));
        h.toggle("Notepad");
        let ranked = h.rank("  ", &apps());
        assert_eq!(&ranked[..2], &[3, 2]);
        h.toggle("Notepad");
        assert_eq!(h.rank("", &apps())[0], 2);
        let persisted = serde_json::to_string(&h).unwrap();
        let restored: History = serde_json::from_str(&persisted).unwrap();
        assert_eq!(restored.rank("", &apps()), h.rank("", &apps()));
    }
    #[test]
    fn most_used_apps_come_first_and_hidden_apps_leave_the_list() {
        let mut h = History::default();
        h.launched("Notepad");
        h.launched("Notepad");
        h.launched("Windows Terminal");
        assert_eq!(&h.rank("", &apps())[..2], &[3, 2]);
        h.set_hidden("Notepad", true);
        assert!(!h.rank("", &apps()).contains(&3));
        assert!(h.hidden("Notepad"));
        let old: History = serde_json::from_str(
            r#"{"counter":2,"items":{"Notepad":{"favorite":false,"last_used":2}}}"#,
        )
        .unwrap();
        assert_eq!(old.rank("", &apps())[0], 3);
    }
}
