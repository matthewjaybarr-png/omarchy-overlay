//! Windows that always float, checked before app rules: file-copy progress,
//! replace and confirm prompts, Properties, Open With and Run. Each built-in
//! entry can be switched off; config\always-float.json adds your own windows.
use serde::{Deserialize, Serialize};

pub const EXAMPLE: &str = include_str!("../config/examples/always-float.json");

pub const BUILTINS: &[(&str, &str)] = &[
    ("run", "Run box"),
    ("open-with", "Open With"),
    ("properties", "Properties windows"),
    ("file-operations", "File copy, move and delete progress"),
    (
        "confirm-prompts",
        "Replace, delete and other confirm prompts",
    ),
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Entry {
    pub executable: String,
    pub window_class: String,
    pub title_contains: String,
}
impl Entry {
    fn matches(&self, exe: &str, title: &str, class: &str) -> bool {
        (self.executable.is_empty() || self.executable.eq_ignore_ascii_case(exe))
            && (self.window_class.is_empty() || self.window_class == class)
            && (self.title_contains.is_empty()
                || title
                    .to_lowercase()
                    .contains(&self.title_contains.to_lowercase()))
    }
    pub fn label(&self) -> String {
        let mut parts = vec![];
        if !self.executable.is_empty() {
            parts.push(self.executable.clone());
        }
        if !self.window_class.is_empty() {
            parts.push(format!("class {}", self.window_class));
        }
        if !self.title_contains.is_empty() {
            parts.push(format!("title has \"{}\"", self.title_contains));
        }
        parts.join(" · ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct List {
    /// Built-in entries the user switched off, by id.
    pub disabled: Vec<String>,
    pub windows: Vec<Entry>,
}
impl List {
    pub fn enabled(&self, id: &str) -> bool {
        !self.disabled.iter().any(|d| d == id)
    }
    /// The reason a window floats, or None when app rules decide.
    pub fn matches(&self, exe: &str, title: &str, class: &str) -> Option<String> {
        let builtin = |id: &str| -> bool {
            match id {
                "run" => {
                    exe.eq_ignore_ascii_case("explorer.exe") && class == "#32770" && title == "Run"
                }
                "open-with" => exe.eq_ignore_ascii_case("OpenWith.exe"),
                "properties" => title.to_lowercase().ends_with("properties"),
                "file-operations" => class == "OperationStatusWindow",
                "confirm-prompts" => class == "#32770",
                _ => false,
            }
        };
        for (id, label) in BUILTINS {
            if self.enabled(id) && builtin(id) {
                return Some(format!("Always-float list: {label}"));
            }
        }
        self.windows
            .iter()
            .find(|e| e.matches(exe, title, class))
            .map(|e| format!("Always-float list: {}", e.label()))
    }
    pub fn validate(&self) -> Result<(), String> {
        if let Some(id) = self
            .disabled
            .iter()
            .find(|d| !BUILTINS.iter().any(|(id, _)| id == d))
        {
            return Err(format!("\"{id}\" is not a built-in always-float entry."));
        }
        if self.windows.len() > 128 {
            return Err("Up to 128 always-float windows are supported.".into());
        }
        for e in &self.windows {
            if e.executable.is_empty() && e.window_class.is_empty() && e.title_contains.is_empty() {
                return Err(
                    "Each always-float window needs a program, a window class or title text."
                        .into(),
                );
            }
            if [&e.executable, &e.window_class, &e.title_contains]
                .iter()
                .any(|s| s.len() > 255 || s.chars().any(char::is_control))
                || e.executable.contains(['\\', '/', ':'])
            {
                return Err(format!(
                    "The always-float entry \"{}\" is not valid.",
                    e.label()
                ));
            }
        }
        Ok(())
    }
}

pub fn parse(text: &str) -> Result<List, String> {
    let list: List = serde_json::from_str(text)
        .map_err(|e| format!("always-float.json line {}: {e}", e.line().max(1)))?;
    list.validate()
        .map_err(|e| format!("always-float.json: {e}"))?;
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_dialogs_float_and_entries_can_be_switched_off() {
        let mut list = parse(EXAMPLE).unwrap();
        assert!(list
            .matches("explorer.exe", "Copying 3 items", "OperationStatusWindow")
            .is_some());
        assert!(list
            .matches("explorer.exe", "Run", "#32770")
            .unwrap()
            .contains("Run box"));
        assert!(list
            .matches("notepad.exe", "Font Properties", "Notepad")
            .is_some());
        assert!(list.matches("OpenWith.exe", "", "Shell_Flyout").is_some());
        assert!(list
            .matches("notepad.exe", "notes.txt - Notepad", "Notepad")
            .is_none());
        list.disabled.push("properties".into());
        assert!(list
            .matches("notepad.exe", "Font Properties", "Notepad")
            .is_none());
        list.windows.push(Entry {
            executable: "calc.exe".into(),
            ..Default::default()
        });
        assert!(list.matches("CALC.EXE", "Calculator", "").is_some());
        assert!(parse("{\"disabled\": [\"nope\"]}").is_err());
        assert!(parse("{\n\"windows\": [{\"executable\": 3}]}")
            .unwrap_err()
            .contains("line 2"));
        assert!(parse("{\"windows\": [{}]}").is_err());
    }
    #[test]
    fn dialogs_seen_on_the_windows_11_test_vm_float() {
        // Program, title and class exactly as the 0.16 VM probe read them.
        let list = parse(EXAMPLE).unwrap();
        let reason = |exe, title, class| list.matches(exe, title, class).unwrap();
        assert!(reason("explorer.exe", "Run", "#32770").contains("Run box"));
        assert!(reason("powershell.exe", "small Properties", "#32770").contains("Properties"));
        assert!(
            reason("powershell.exe", "51% complete", "OperationStatusWindow").contains("progress")
        );
        assert!(reason(
            "powershell.exe",
            "Replace or Skip Files",
            "OperationStatusWindow"
        )
        .contains("progress"));
    }
}
