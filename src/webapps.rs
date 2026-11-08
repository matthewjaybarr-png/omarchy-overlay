//! config\webapps.json: web apps that keybinds can bring to the front.
//! Omarchy Overlay looks for an open window first, then the Chrome Apps shortcut of
//! the same name, and only offers to open the site when neither exists.
use serde::{Deserialize, Serialize};

pub const DEFAULT: &str = include_str!("../config/examples/webapps.json");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebApp {
    pub name: String,
    pub url: String,
    /// The Start-menu shortcut name Chrome or Edge made when the app was installed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shortcut: String,
    /// Text in the open window's title. Left out, the name is used.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub window_title: String,
}
impl WebApp {
    pub fn shortcut(&self) -> &str {
        if self.shortcut.is_empty() {
            &self.name
        } else {
            &self.shortcut
        }
    }
    pub fn window_title(&self) -> &str {
        if self.window_title.is_empty() {
            &self.name
        } else {
            &self.window_title
        }
    }
    /// Whether an open window is this app (0.19). `label` is the bar's name for the window,
    /// which for a Chrome or Edge app window is its Start-menu shortcut ("ChatGPT"). The
    /// title must be the app's title exactly or end with " - <title>", so a mail, a browser
    /// tab or a chat that only mentions "ChatGPT" is no longer brought forward.
    pub fn matches(&self, label: &str, title: &str) -> bool {
        let wanted = self.window_title().trim().to_lowercase();
        let title = title.trim().to_lowercase();
        label.eq_ignore_ascii_case(self.shortcut())
            || label.eq_ignore_ascii_case(&self.name)
            || title == wanted
            || title.ends_with(&format!(" - {wanted}"))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    apps: Vec<WebApp>,
}

pub fn parse(text: &str) -> Result<Vec<WebApp>, String> {
    let file: File = serde_json::from_str(text)
        .map_err(|e| format!("webapps.json line {}: {e}", e.line().max(1)))?;
    if file.apps.len() > 64 {
        return Err("webapps.json: up to 64 web apps are supported.".into());
    }
    for (i, app) in file.apps.iter().enumerate() {
        let line = text
            .lines()
            .position(|l| l.contains(&serde_json::to_string(&app.name).unwrap_or_default()))
            .map_or(1, |n| n + 1);
        let at = |m: &str| format!("webapps.json line {line}: {m}");
        let text_ok =
            |s: &str, max: usize| s.chars().count() <= max && !s.chars().any(char::is_control);
        if app.name.trim().is_empty() || !text_ok(&app.name, 80) {
            return Err(at("the name must be 1 to 80 characters."));
        }
        if file.apps[..i]
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case(&app.name))
        {
            return Err(at("this name is used twice."));
        }
        if !app.url.starts_with("https://")
            || !text_ok(&app.url, 2048)
            || app.url.contains([' ', '"'])
        {
            return Err(at(
                "the address must start with https:// and have no spaces.",
            ));
        }
        if !text_ok(&app.shortcut, 120)
            || app
                .shortcut
                .contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|'])
            || !text_ok(&app.window_title, 120)
        {
            return Err(at("the shortcut name or window title is not valid."));
        }
    }
    Ok(file.apps)
}

/// The file text for a list of web apps, used when one is removed from the menu.
pub fn to_text(apps: &[WebApp]) -> String {
    serde_json::to_string_pretty(&File {
        apps: apps.to_vec(),
    })
    .unwrap_or_default()
        + "\n"
}

/// True for a picker entry that opens a Chrome or Edge web app: a shortcut in a
/// "Chrome Apps" or "Edge Apps" Start-menu folder, or an Apps-folder ID with "_crx_".
pub fn browser_app(path: &std::path::Path, arguments: &str) -> bool {
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    folder.ends_with(" apps") && (folder.contains("chrome") || folder.contains("edge"))
        || arguments.to_ascii_lowercase().contains("_crx_")
}

/// Removes picker entries with the same name, keeping the first (0.19). A web app and a
/// real program with the same name ("Claude" from Chrome and the Claude desktop app) are
/// both kept, and the web app is shown as "<name> (web app)". `entries` must be sorted by
/// name; the result is too.
pub fn dedupe<T>(
    entries: Vec<T>,
    label: impl Fn(&T) -> &str,
    web: impl Fn(&T) -> bool,
    mark_web: impl Fn(&mut T),
) -> Vec<T> {
    let mut kept: Vec<T> = Vec::with_capacity(entries.len());
    let mut group = 0;
    for entry in entries {
        if kept
            .last()
            .is_none_or(|last| !label(last).eq_ignore_ascii_case(label(&entry)))
        {
            group = kept.len();
        }
        if kept[group..].iter().any(|k| web(k) == web(&entry)) {
            continue;
        }
        kept.push(entry);
    }
    let shared: Vec<bool> = (0..kept.len())
        .map(|i| {
            let same = |j: usize| label(&kept[j]).eq_ignore_ascii_case(label(&kept[i]));
            (i > 0 && same(i - 1)) || (i + 1 < kept.len() && same(i + 1))
        })
        .collect();
    for (entry, shared) in kept.iter_mut().zip(shared) {
        if shared && web(entry) {
            mark_web(entry);
        }
    }
    kept.sort_by_key(|e| label(e).to_lowercase());
    kept
}

pub fn defaults() -> Vec<WebApp> {
    parse(DEFAULT).expect("built-in webapps.json is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_apps_need_https_and_report_lines() {
        let apps = defaults();
        assert_eq!(apps[0].name, "ChatGPT");
        assert_eq!(apps[1].shortcut(), "Claude");
        let err = parse("{\"apps\": [\n{\"name\": \"X\", \"url\": \"http://x\"}]}").unwrap_err();
        assert!(err.contains("line 2") && err.contains("https"), "{err}");
        assert!(parse(
            "{\"apps\": [{\"name\": \"X\", \"url\": \"https://x\", \"shortcut\": \"..\\\\a\"}]}"
        )
        .is_err());
        assert_eq!(parse(&to_text(&apps[1..])).unwrap(), apps[1..]);
    }
    #[test]
    fn only_the_app_window_matches_not_any_title_that_mentions_it() {
        let chatgpt = &defaults()[0];
        assert!(chatgpt.matches("ChatGPT", "Rust icon question"));
        assert!(chatgpt.matches("Chrome", "ChatGPT"));
        assert!(chatgpt.matches("Chrome", "New chat - ChatGPT"));
        assert!(!chatgpt.matches("Outlook", "RE: ChatGPT licence - Outlook"));
        assert!(!chatgpt.matches("Chrome", "ChatGPT pricing - Google Search - Google Chrome"));
        assert!(!chatgpt.matches("Claude", "Compare ChatGPT and Claude"));
    }
    #[test]
    fn picker_keeps_a_program_and_a_web_app_with_the_same_name() {
        use std::path::Path;
        let chrome = Path::new("C:/Users/m/AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Chrome Apps/Claude.lnk");
        assert!(browser_app(chrome, ""));
        assert!(browser_app(Path::new("explorer.exe"), "\"shell:AppsFolder\\Chrome._crx_fmpn\""));
        assert!(!browser_app(Path::new("explorer.exe"), "\"shell:AppsFolder\\AnthropicPBC.Claude_x!Claude\""));
        // (name, web app)
        let entries = vec![
            ("ChatGPT".to_string(), true),
            ("chatgpt".to_string(), true),
            ("Claude".to_string(), true),
            ("Claude".to_string(), false),
            ("Claude".to_string(), true),
            ("Claude Code".to_string(), false),
            ("Notepad".to_string(), false),
            ("Notepad".to_string(), false),
        ];
        let kept = dedupe(entries, |e| e.0.as_str(), |e| e.1, |e| e.0.push_str(" (web app)"));
        let names: Vec<&str> = kept.iter().map(|e| e.0.as_str()).collect();
        assert_eq!(names, ["ChatGPT", "Claude", "Claude (web app)", "Claude Code", "Notepad"]);
    }
}
