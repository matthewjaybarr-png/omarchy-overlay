//! Omarchy's reminders (Super+Ctrl+R): a Windows notification after a set number of minutes.
//! shortcut: reminders live in memory and are lost on exit or restart; save them if users ask.
use super::*;
use std::time::{Duration, Instant};

/// "in 4 min" for the time left until `due`, rounded up.
fn left(due: Instant, now: Instant) -> String {
    let minutes = due.saturating_duration_since(now).as_secs().div_ceil(60);
    format!("{minutes} min")
}

impl App {
    pub(super) unsafe fn remind(&mut self, minutes: u32) {
        let due = Instant::now() + Duration::from_secs(u64::from(minutes) * 60);
        self.reminders.push((due, minutes));
        self.show_feedback(&format!("Reminder in {minutes} min"));
    }
    pub(super) unsafe fn show_reminders(&self) {
        let now = Instant::now();
        let mut due: Vec<Instant> = self.reminders.iter().map(|(d, _)| *d).collect();
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
        self.show_feedback("Reminders cleared");
    }
    /// Run on the main timer: a due reminder shows as a Windows notification from the tray icon.
    pub(super) unsafe fn check_reminders(&mut self) {
        let now = Instant::now();
        let (due, waiting): (Vec<_>, Vec<_>) = self.reminders.iter().partition(|(d, _)| *d <= now);
        if due.is_empty() {
            return;
        }
        self.reminders = waiting;
        for (_, minutes) in due {
            let mut tray = self.tray;
            tray.uFlags = NIF_INFO;
            tray.dwInfoFlags = NIIF_INFO;
            copy_wide(&mut tray.szInfoTitle, "Reminder");
            copy_wide(&mut tray.szInfo, &format!("{minutes} minutes are up."));
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
        let now = std::time::Instant::now();
        assert_eq!(super::left(now + std::time::Duration::from_secs(61), now), "2 min");
        assert_eq!(super::left(now, now), "0 min");
        let mut buffer = [1u16; 4];
        super::copy_wide(&mut buffer, "abcdef");
        assert_eq!(buffer, [97, 98, 99, 0]);
    }
}
