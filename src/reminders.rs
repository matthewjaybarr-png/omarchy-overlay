//! Omarchy's reminders (Super+Ctrl+R, omarchy-reminder): a Windows notification after a
//! set number of minutes, with an optional message. They are kept in reminders.json, so a
//! reminder set before a restart or a sign-out still comes, late if the PC was off.
use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

const FILE: &str = "reminders.json";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reminder {
    /// When it is due, in seconds since 1970 (wall clock, so it survives a restart).
    due: u64,
    minutes: u32,
    #[serde(default)]
    message: String,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}
/// "4 min" for the time left until `due`, rounded up.
fn left(due: u64, now: u64) -> String {
    format!("{} min", due.saturating_sub(now).div_ceil(60))
}
/// The notification text for a reminder.
fn text(r: &Reminder) -> String {
    if r.message.is_empty() {
        format!("{} minutes are up.", r.minutes)
    } else {
        r.message.clone()
    }
}
pub(super) fn load(directory: &Path) -> Vec<Reminder> {
    let path = directory.join(FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![];
    };
    match serde_json::from_str::<Vec<Reminder>>(&text) {
        Ok(list) => list.into_iter().take(64).collect(),
        Err(error) => {
            log(&format!("{FILE}: {error}; reminders were not loaded."));
            vec![]
        }
    }
}

impl App {
    fn save_reminders(&self) {
        let path = self.directory.join(FILE);
        let result = if self.reminders.is_empty() {
            if path.exists() {
                fs::remove_file(&path).map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        } else {
            write_json_atomic(&path, &self.reminders)
        };
        if let Err(error) = result {
            log(&format!("Reminders: {error}"));
        }
    }
    pub(super) unsafe fn remind(&mut self, minutes: u32, message: &str) {
        if self.reminders.len() >= 64 {
            self.show_feedback("Up to 64 reminders at a time");
            return;
        }
        self.reminders.push(Reminder {
            due: now() + u64::from(minutes) * 60,
            minutes,
            message: message.into(),
        });
        self.save_reminders();
        self.show_feedback(&format!("Reminder in {minutes} min"));
    }
    pub(super) unsafe fn show_reminders(&self) {
        let now = now();
        let mut due: Vec<u64> = self.reminders.iter().map(|r| r.due).collect();
        due.sort();
        let text = if due.is_empty() {
            "No reminders".to_string()
        } else {
            let times: Vec<String> = due.iter().map(|d| left(*d, now)).collect();
            format!("Reminders in {}", times.join(", "))
        };
        self.show_feedback(&text);
    }
    pub(super) unsafe fn clear_reminders(&mut self) {
        self.reminders.clear();
        self.save_reminders();
        self.show_feedback("Reminders cleared");
    }
    /// Run on the main timer: a due reminder shows as a Windows notification from the tray icon.
    pub(super) unsafe fn check_reminders(&mut self) {
        let now = now();
        let (due, waiting): (Vec<_>, Vec<_>) = self.reminders.iter().cloned().partition(|r| r.due <= now);
        if due.is_empty() {
            return;
        }
        self.reminders = waiting;
        self.save_reminders();
        for reminder in due {
            let mut tray = self.tray;
            tray.uFlags = NIF_INFO;
            tray.dwInfoFlags = NIIF_INFO;
            copy_wide(&mut tray.szInfoTitle, "Reminder");
            copy_wide(&mut tray.szInfo, &text(&reminder));
            Shell_NotifyIconW(NIM_MODIFY, &tray);
        }
    }
}

/// Copies `text` into a fixed, zero-terminated UTF-16 buffer, cut to fit.
fn copy_wide(buffer: &mut [u16], text: &str) {
    let text: Vec<u16> = text.encode_utf16().take(buffer.len() - 1).collect();
    buffer[..text.len()].copy_from_slice(&text);
    buffer[text.len()] = 0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn time_left_rounds_up() {
        assert_eq!(super::left(1061, 1000), "2 min");
        assert_eq!(super::left(1000, 1000), "0 min");
        let mut buffer = [1u16; 4];
        super::copy_wide(&mut buffer, "abcdef");
        assert_eq!(buffer, [97, 98, 99, 0]);
    }
}
