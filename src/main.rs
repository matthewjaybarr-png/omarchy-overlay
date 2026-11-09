#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod always_float;
#[cfg(any(windows, test))]
mod appearance_profiles;
#[cfg(any(windows, test))]
mod bar_model;
#[cfg(any(windows, test))]
mod binds;
#[cfg(any(windows, test))]
mod commands;
#[cfg(any(windows, test))]
mod config;
#[cfg(any(windows, test))]
mod control_theme;
#[cfg(any(windows, test))]
mod desktops;
#[cfg(any(windows, test))]
mod dwindle;
#[cfg(any(windows, test))]
mod effects_gate;
#[cfg(any(windows, test))]
mod frame_colors;
#[cfg(any(windows, test))]
mod font_names;
#[cfg(any(windows, test))]
mod install;
#[cfg(any(windows, test))]
mod launch_unelevated;
#[cfg(any(windows, test))]
mod layout;
#[cfg(any(windows, test))]
mod material;
#[cfg(any(windows, test))]
mod menu_model;
#[cfg(any(windows, test))]
mod motion;
#[cfg(any(windows, test))]
mod omarchy_themes;
#[cfg(any(windows, test))]
mod picker;
#[cfg(any(windows, test))]
mod portable_apps;
#[cfg(any(windows, test))]
mod profile;
#[cfg(test)]
mod rainmeter;
#[cfg(any(windows, test))]
mod rules;
#[cfg(any(windows, test))]
mod scrolling;
#[cfg(any(windows, test))]
mod shortcuts;
#[cfg(any(windows, test))]
mod splits;
#[cfg(any(windows, test))]
mod theme_files;
#[cfg(any(windows, test))]
mod tray_match;
#[cfg(any(windows, test))]
mod webapps;
#[cfg(windows)]
mod windows;
#[cfg(any(windows, test))]
mod workspace_layouts;

fn main() {
    #[cfg(windows)]
    windows::run();
    #[cfg(not(windows))]
    eprintln!("Omarchy Overlay runs on Windows 11. Use cargo test here to verify layout calculations.");
}
