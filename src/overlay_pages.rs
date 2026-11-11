//! What the Omarchy Overlay menu shows. Every level is rebuilt from App whenever it opens or
//! changes, so a page never shows a stale value. Changes go through `commit`, which
//! validates, saves the profile and applies only what changed.
use super::overlay::{self, Act, Glyph, Level, Row, View};
use super::*;
use crate::appearance_profiles::{self, Profile};
use crate::commands::{self, Command};
use crate::rules::{AppRule, Mode};

const MENU_HINT: &str = "Enter opens · Esc goes back · type to search";
const PAGE_HINT: &str = "Enter chooses · Esc goes back · type to search";
/// Real app icons loaded per app picker view; rows further down show a plain glyph.
const ICON_BUDGET: usize = 48;

// Nerd Font glyphs from the bundled JetBrainsMono Nerd Font (Font Awesome range).
const INFO: char = '\u{f05a}';
const UNDO: char = '\u{f0e2}';
const DEFAULTS: char = '\u{f021}';
const WARNING: char = '\u{f071}';
const COG: char = '\u{f013}';
const BRUSH: char = '\u{f1fc}';
const FONT: char = '\u{f031}';
const IMAGE: char = '\u{f03e}';
const WINDOW: char = '\u{f2d0}';
const FLOAT: char = '\u{f2d2}';
const CLOSE: char = '\u{f00d}';
const KEYBOARD: char = '\u{f11c}';
const SAVE: char = '\u{f0c7}';
const TRASH: char = '\u{f1f8}';
const PLUS: char = '\u{f067}';
const FOLDER: char = '\u{f07b}';
const WINDOWS: char = '\u{f17a}';
const DESKTOP: char = '\u{f108}';
const GLOBE: char = '\u{f0ac}';
const OPACITY: char = '\u{f042}';
const GRID: char = '\u{f00a}';
const CLOCK: char = '\u{f017}';
const SLIDERS: char = '\u{f1de}';
const SUBMENU: char = '\u{f105}';
const RUN: char = '\u{f04b}';
const MOON: char = '\u{f186}';
const SUN: char = '\u{f185}';
const OPTION: char = '\u{f10c}';
const POWER: char = '\u{f011}';
const STAR: char = '\u{f005}';
const EDIT: char = '\u{f044}';

/// The settings before the first change on a page, for "Undo changes".
pub(super) struct Undo {
    page: &'static str,
    config: Config,
    webapps: Vec<crate::webapps::WebApp>,
    picker: crate::picker::History,
}

/// Pages drawn by the 0.15 windows because they need text boxes or drag and drop.
/// The three bar sections, each its own menu page.
const BAR_SECTIONS: [&str; 3] = ["bar-left", "bar-center", "bar-right"];
/// Pages that take typed text: what is typed stays visible and Enter saves it.
const TEXT_PAGES: [&str; 5] = [
    "app-new",
    "app-name",
    "app-target",
    "app-args",
    "app-folder",
];

fn info(text: impl Into<String>) -> Row {
    Row::new(INFO, text, Act::None)
}
fn task_row(task: &tasks::Task, label: String, act: Act) -> Row {
    let mut row = Row::new(WINDOW, label, act);
    row.icon = Glyph::Task(task.identity.clone());
    row
}
fn page(title: &str, rows: Vec<Row>) -> View {
    View {
        title: title.into(),
        rows,
        hint: PAGE_HINT.into(),
        ranked: false,
    }
}
fn page_title(id: &str) -> &str {
    commands::PAGES
        .iter()
        .find(|(p, _)| *p == id)
        .map_or(id, |(_, label)| label)
}
/// One row per value; the current one is checked.
fn values<T: PartialEq + Copy>(
    page: &'static str,
    icon: char,
    current: T,
    list: &[(T, String)],
) -> Vec<Row> {
    list.iter()
        .map(|(value, label)| {
            Row::new(
                icon,
                label.clone(),
                Act::Choose(page, index_of(list, *value)),
            )
            .checked(*value == current)
        })
        .collect()
}
fn index_of<T: PartialEq>(list: &[(T, String)], value: T) -> String {
    list.iter()
        .position(|(v, _)| *v == value)
        .unwrap_or(0)
        .to_string()
}
fn display_mode(label: &str) -> Option<crate::bar_model::DisplayMode> {
    crate::bar_model::DisplayMode::ALL
        .into_iter()
        .find(|m| m.label() == label)
}
fn on_off(page: &'static str, on: bool, on_label: &str, off_label: &str) -> Vec<Row> {
    vec![
        Row::new(OPTION, on_label, Act::Choose(page, "on".into())).checked(on),
        Row::new(OPTION, off_label, Act::Choose(page, "off".into())).checked(!on),
    ]
}
fn mode_of(value: &str) -> Mode {
    match value {
        "Float" => Mode::Float,
        "Ignore" => Mode::Ignore,
        _ => Mode::Tile,
    }
}
fn mode_rows(page: &'static str, current: Mode) -> Vec<Row> {
    [
        (Mode::Tile, "Tile: Omarchy Overlay arranges it"),
        (Mode::Float, "Float: Omarchy Overlay leaves it where it is"),
        (Mode::Ignore, "Ignore: Omarchy Overlay never touches it"),
    ]
    .into_iter()
    .map(|(mode, label)| {
        Row::new(GRID, label, Act::Choose(page, mode.label().into())).checked(mode == current)
    })
    .collect()
}
/// Steps through a short list of choices, wrapping at the end.
fn cycle<T: PartialEq + Clone>(current: &T, list: &[T]) -> T {
    let next = list
        .iter()
        .position(|v| v == current)
        .map_or(0, |i| (i + 1) % list.len());
    list[next].clone()
}
fn percent(value: Option<u8>) -> String {
    value.map_or("app default".into(), |v| format!("{v}%"))
}
pub(super) fn hex(color: u32) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        color & 255,
        (color >> 8) & 255,
        (color >> 16) & 255
    )
}
fn material_name(m: &str) -> &'static str {
    match m {
        "mica" => "Mica",
        "acrylic" => "Acrylic",
        _ => "Plain color",
    }
}
fn material_label(m: &str) -> &'static str {
    match m {
        "mica" => "Mica: tinted by the background (Windows 11)",
        "acrylic" => "Acrylic: frosted blur (Windows 11)",
        _ => "Plain color",
    }
}
fn differs<T: serde::Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() != serde_json::to_value(b).ok()
}
/// Resets the settings a page shows. False for pages without plain defaults.
fn page_defaults(page: &str, c: &mut Config) -> bool {
    let d = Config::default();
    match page {
        "theme" => c.theme = d.theme,
        "font" => {
            c.menu_font = d.menu_font;
            c.menu_font_size = d.menu_font_size;
        }
        "gaps" => c.gap = d.gap,
        "main-size" => c.main_ratio = d.main_ratio,
        "layout" => c.layout = d.layout,
        "columns" => c.scrolling_columns = d.scrolling_columns,
        "opacity" => c.active_opacity = d.active_opacity,
        "inactive-opacity" => c.inactive_opacity = d.inactive_opacity,
        "materials" => {
            c.window_backdrop = d.window_backdrop;
            c.menu_backdrop = d.menu_backdrop;
        }
        "app-materials" => c.window_backdrop = d.window_backdrop,
        "menu-materials" => c.menu_backdrop = d.menu_backdrop,
        "animations" => {
            c.animations = d.animations;
            c.animation_duration_ms = d.animation_duration_ms;
        }
        "focus-outline" => c.focus_outline = d.focus_outline,
        "osd" => c.show_osd = d.show_osd,
        "start-tiling" => c.start_tiling = d.start_tiling,
        "bar" => c.show_top_bar = d.show_top_bar,
        "clock" => c.bar.clock_format = d.bar.clock_format,
        "bar-style" => c.bar.module_display = d.bar.module_display,
        "bar-items" => c.bar.reset_layout(),
        "bar-height" => {
            c.bar.height = d.bar.height;
            c.bar.font_size = d.bar.font_size;
        }
        "bar-opacity" => c.bar_opacity = d.bar_opacity,
        "taskbar" => {
            c.taskbar_mode = d.taskbar_mode;
            c.auto_hide_taskbar = d.auto_hide_taskbar;
        }
        "keymap" => c.keymap = d.keymap,
        "tap-super" => c.tap_super = d.tap_super,
        "keyboard" => {
            c.modifiers = d.modifiers;
            c.keymap = d.keymap;
            c.tap_super = d.tap_super;
            c.shortcut_overrides.clear();
        }
        "conflicts" => c.shortcut_overrides.clear(),
        "app-rules" => c.rules.clear(),
        "default-rule" => {
            c.default_mode = d.default_mode;
            c.default_allow_opacity = d.default_allow_opacity;
        }
        "always-float" => c.always_float = Default::default(),
        _ => return false,
    }
    true
}
fn has_defaults(page: &str) -> bool {
    page_defaults(page, &mut Config::default())
        || matches!(
            page,
            "background" | "windows-theme" | "webapps" | "install-webapp" | "remove-apps" | "rule"
        )
}
/// The settings key of a level, for Undo and Restore defaults.
fn level_key(level: &Level) -> &'static str {
    match level {
        Level::Page(p) => p,
        Level::Rule(Some(_)) => "rule",
        Level::Rule(None) => "default-rule",
        Level::Profile(_) => "profile",
        Level::WindowPanel(_) => "panel",
        _ => "",
    }
}
fn wallpapers(folders: &[PathBuf]) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = vec![];
    for folder in folders {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|x| {
                    ["jpg", "jpeg", "png", "bmp"]
                        .iter()
                        .any(|e| x.eq_ignore_ascii_case(e))
                })
            })
            .collect();
        paths.sort();
        for path in paths {
            if !found.iter().any(|f| f.file_name() == path.file_name()) {
                found.push(path);
            }
        }
    }
    found.truncate(64);
    found
}

impl App {
    /// The window the menu acts on: the one that was in front before the menu opened.
    pub(super) unsafe fn menu_target(&self) -> HWND {
        let foreground = GetForegroundWindow();
        if overlay::visible() || foreground == overlay::hwnd() {
            overlay::target()
        } else {
            foreground
        }
    }
    /// Opens a level. `fresh` starts a new menu, and pressing the same key again closes it.
    pub(super) unsafe fn open_level(&mut self, level: Level, fresh: bool) {
        if fresh
            && overlay::visible()
            && overlay::depth() == 1
            && overlay::level().as_ref() == Some(&level)
        {
            overlay::hide();
            return;
        }
        let target = self.menu_target();
        if level == Level::Apps && (self.command_menu || self.entries.is_empty()) {
            self.command_menu = false;
            self.refresh_apps();
        }
        self.confirm_choice = None;
        let view = self.view(&level);
        overlay::open(level, view, target, fresh);
    }
    pub(super) unsafe fn open_menu(&mut self, id: &str) {
        self.open_level(Level::Menu(id.into()), true);
    }
    unsafe fn push(&mut self, level: Level) {
        self.open_level(level, false);
    }
    unsafe fn refresh_view(&mut self) {
        if let Some(level) = overlay::level() {
            let view = self.view(&level);
            overlay::set_view(view);
        }
    }
    unsafe fn view(&mut self, level: &Level) -> View {
        match level {
            Level::Menu(id) => self.menu_view(id),
            Level::Page("default-rule") => self.rule_view(None),
            Level::Page(id) => self.page_view(id),
            Level::Apps => self.apps_view(),
            Level::Keybinds => self.keybinds_view(),
            Level::Windows => windows_view(),
            Level::WindowPanel(h) => self.window_panel_view(*h as HWND),
            Level::Confirm(_, question) => View {
                title: question.clone(),
                rows: vec![
                    Row::new(POWER, "Yes", Act::Choose("confirm", "yes".into())),
                    Row::new(CLOSE, "No, go back", Act::Back),
                ],
                hint: "Enter chooses · Esc goes back".into(),
                ranked: false,
            },
            Level::Profile(i) => self.profile_view(*i),
            Level::Rule(i) => self.rule_view(*i),
        }
    }
    /// The checked state shown beside toggle commands in a menu.
    fn command_state(&self, command: &Command) -> Option<bool> {
        match command {
            Command::Builtin("keep-awake") => Some(self.keep_awake),
            Command::Builtin("toggle-bar") => Some(self.config.show_top_bar),
            Command::Builtin("pause-keys") => Some(self.keys_paused),
            Command::Action(i) if crate::shortcuts::ACTIONS[*i].id == "toggle-tiling" => {
                Some(self.active)
            }
            _ => None,
        }
    }
    fn menu_view(&self, id: &str) -> View {
        let Some(menu) = self.menus.get(id) else {
            return View {
                title: "Menu".into(),
                rows: vec![info(format!(
                    "menu.json has no list called \"{id}\". Esc goes back."
                ))],
                hint: MENU_HINT.into(),
                ranked: false,
            };
        };
        let mut rows = vec![];
        if id == "main" && !self.config_errors.is_empty() {
            rows.push(
                Row::new(
                    WARNING,
                    format!(
                        "{} config file problem(s): built-in versions in use",
                        self.config_errors.len()
                    ),
                    Act::Open(Level::Page("config-errors")),
                )
                .detail("Enter shows them"),
            );
        }
        if id == "main" && self.keys_paused {
            rows.push(
                Row::new(
                    KEYBOARD,
                    "Omarchy Overlay keys are paused",
                    Act::Run(Command::Builtin("pause-keys")),
                )
                .detail("Enter resumes"),
            );
        }
        for item in &menu.items {
            let submenu = matches!(item.command, Command::Menu(_) | Command::Page(_));
            let act = if item.confirm {
                Act::Confirm(item.command.clone(), format!("{}?", item.label))
            } else {
                Act::Run(item.command.clone())
            };
            let mut row = Row::new(
                item.icon.unwrap_or(if submenu { SUBMENU } else { RUN }),
                item.label.clone(),
                act,
            )
            .windows(item.windows || item.command.windows());
            if let Some(on) = self.command_state(&item.command) {
                row = row.checked(on).detail(if on { "on" } else { "off" });
            } else if submenu {
                row = row.detail("›");
            }
            rows.push(row);
        }
        View {
            title: menu.title.clone(),
            rows,
            hint: MENU_HINT.into(),
            ranked: false,
        }
    }
    unsafe fn apps_view(&mut self) -> View {
        let apps: Vec<(String, String)> = self
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.key()))
            .collect();
        let order = self.picker_history.rank(&overlay::filter(), &apps);
        let mut rows = Vec::with_capacity(order.len());
        for (n, &i) in order.iter().enumerate() {
            let key = self.entries[i].key();
            let mut row = Row::new(RUN, self.entries[i].label.clone(), Act::Launch(i));
            if n < ICON_BUDGET {
                row.icon = Glyph::Icon(self.entry_icon(i));
            }
            if self.picker_history.favorite(&key) {
                row = row.detail("favorite");
            } else if self.picker_history.recent(&key) {
                row = row.detail("recent");
            }
            rows.push(row);
        }
        if rows.is_empty() {
            rows.push(info("No app matches. Backspace clears the search."));
        }
        View {
            title: "Apps".into(),
            rows,
            hint: "Enter launches · type to search · Esc closes".into(),
            ranked: true,
        }
    }
    pub(super) unsafe fn overlay_filter_changed(&mut self) {
        match overlay::level() {
            Some(Level::Apps) => {
                let view = self.apps_view();
                overlay::set_view(view);
            }
            Some(Level::Page(p)) if TEXT_PAGES.contains(&p) => {
                let view = self.text_view(p);
                overlay::set_view(view);
            }
            _ => {}
        }
    }
    /// A page that takes typed text: the first row saves what is typed (0.17, 3.3).
    fn text_view(&self, id: &'static str) -> View {
        let typed = overlay::filter();
        let entry = self
            .custom_app
            .and_then(|i| self.config.applications.get(i));
        let (what, current, optional) = match id {
            "app-new" => (
                "full or relative path of the app, file or folder",
                String::new(),
                false,
            ),
            "app-name" => (
                "name",
                entry.map(|e| e.name.clone()).unwrap_or_default(),
                false,
            ),
            "app-target" => (
                "file or folder path",
                entry.map(|e| e.target.clone()).unwrap_or_default(),
                false,
            ),
            "app-args" => (
                "arguments",
                entry.map(|e| e.arguments.clone()).unwrap_or_default(),
                true,
            ),
            _ => (
                "working folder",
                entry
                    .map(|e| e.working_directory.clone())
                    .unwrap_or_default(),
                true,
            ),
        };
        let mut rows = vec![Row::new(
            SAVE,
            if typed.trim().is_empty() {
                "Type it, then press Enter".to_string()
            } else {
                format!("Save: {typed}")
            },
            Act::Choose("app-text", "save".into()),
        )];
        if optional {
            rows.push(Row::new(
                CLOSE,
                "Clear it",
                Act::Choose("app-text", "clear".into()),
            ));
        }
        rows.push(info(format!(
            "Type the {what}. Esc goes back without saving."
        )));
        if !current.is_empty() {
            rows.push(info(format!("Now: {current}")));
        }
        View {
            title: page_title(id).into(),
            rows,
            hint: PAGE_HINT.into(),
            ranked: true,
        }
    }
    /// Saves one field of the custom app being edited, or adds a new one from a path.
    unsafe fn save_custom_app(&mut self, field: &str, text: String) -> Result<String, String> {
        let mut c = self.config.clone();
        // Explorer's "Copy as path" wraps the path in quotes; paths are stored without them.
        let trimmed = crate::menu_model::unquote(text.trim()).to_string();
        // 0.18: paths on the stick are kept relative to OVERLAY SETTINGS (no drive letters).
        let trimmed = match (field, profile::settings()) {
            ("app-new" | "app-target" | "app-folder", Some(settings)) => {
                crate::portable_apps::relative_to_settings(settings, &trimmed)
            }
            _ => trimmed,
        };
        let index = if field == "app-new" {
            if c.applications.len() >= 128 {
                return Err("Up to 128 custom applications are supported.".into());
            }
            let name = Path::new(trimmed.trim_end_matches(['\\', '/']))
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| trimmed.clone());
            c.applications.push(crate::portable_apps::Application {
                id: profile::new_id(),
                name,
                target: trimmed.clone(),
                arguments: String::new(),
                working_directory: String::new(),
            });
            c.applications.len() - 1
        } else {
            let i = self
                .custom_app
                .filter(|&i| i < c.applications.len())
                .ok_or("That entry no longer exists.")?;
            let e = &mut c.applications[i];
            match field {
                "app-name" => e.name = trimmed,
                "app-target" => e.target = trimmed,
                "app-args" => e.arguments = text,
                "app-folder" => e.working_directory = trimmed,
                _ => return Err("Nothing to save.".into()),
            }
            i
        };
        c.applications[index].validate()?;
        let name = c.applications[index].name.clone();
        self.remember("custom-apps");
        if !self.commit(c) {
            return Err("Not saved.".into());
        }
        self.custom_app = Some(index);
        self.refresh_apps();
        Ok(if field == "app-new" {
            format!("Added {name}.")
        } else {
            "Saved.".into()
        })
    }
    fn keybinds_view(&self) -> View {
        let mut rows = vec![];
        if self.keys_paused {
            rows.push(info(
                "Omarchy Overlay keys are paused. Every key belongs to Windows.",
            ));
        }
        if self.config.keymap != "classic" {
            for bind in &self.binds {
                let mut detail = bind.key.label();
                if let Some(windows) = keyhook::replaces(bind.key) {
                    detail = format!("{detail}  (Windows: {windows})");
                }
                rows.push(
                    Row::new(
                        KEYBOARD,
                        bind.description.clone(),
                        Act::Run(bind.command.clone()),
                    )
                    .detail(detail),
                );
            }
        }
        if self.config.keymap != "omarchy" {
            for (i, action) in crate::shortcuts::ACTIONS.iter().enumerate() {
                let chord = crate::shortcuts::chord(
                    i,
                    &self.config.modifiers,
                    &self.config.shortcut_overrides,
                );
                if chord.is_empty() {
                    continue;
                }
                let registered = self
                    .hotkeys
                    .contains(&(BASE as i32 + crate::shortcuts::action_slot(i) as i32));
                rows.push(
                    Row::new(
                        if registered { KEYBOARD } else { WARNING },
                        action.label,
                        Act::Run(Command::Action(i)),
                    )
                    .detail(if registered {
                        chord
                    } else {
                        format!("{chord}  (taken: see Keyboard conflicts)")
                    }),
                );
            }
        }
        View {
            title: "Keybinds".into(),
            rows,
            hint: "Enter runs · type to search · Esc goes back".into(),
            ranked: false,
        }
    }
    unsafe fn page_view(&mut self, id: &'static str) -> View {
        let mut rows = match id {
            "theme" => {
                let mut list: Vec<(String, String, bool)> = theme::NAMES
                    .iter()
                    .map(|(n, l)| {
                        let dark = theme::palette(n).is_some_and(|p| p.dark);
                        ((*n).to_string(), (*l).to_string(), dark)
                    })
                    .collect();
                for t in crate::theme_files::loaded() {
                    if theme::NAMES.iter().any(|(n, _)| *n == t.id) {
                        continue; // replaces the built-in entry above
                    }
                    list.push((t.id.clone(), format!("{} (config\\themes)", t.label), t.dark));
                }
                list.into_iter()
                    .map(|(name, label, dark)| {
                        Row::new(BRUSH, label, Act::Choose("theme", name.clone()))
                            .detail(if dark { "dark" } else { "light" })
                            .checked(self.config.theme == name)
                    })
                    .collect()
            }
            "font" => {
                let mut rows = vec![];
                if !fonts::exists(&self.config.menu_font) {
                    rows.push(
                        Row::new(WARNING, format!("{} is not installed: Segoe UI is used", self.config.menu_font), Act::None)
                            .detail("pick another font or install it"),
                    );
                }
                let names = crate::config::MENU_FONTS;
                let row = |f: String| {
                    let label = names.iter().find(|(n, _)| *n == f).map_or(f.clone(), |(_, l)| (*l).to_string());
                    Row::new(FONT, label, Act::Choose("font", f.clone())).checked(self.config.menu_font == f)
                };
                // JetBrains Mono NL and the Nerd Font first, then the sizes, then the
                // rest of the Windows fonts (type to filter the long list).
                let mut all = fonts::choices().into_iter();
                rows.extend(all.by_ref().take(2).map(row));
                for size in [10, 11, 12, 13, 14, 16] {
                    rows.push(
                        Row::new(FONT, format!("Text size {size}"), Act::Choose("font-size", size.to_string()))
                            .checked(self.config.menu_font_size == size),
                    );
                }
                rows.push(info(match fonts::source() {
                    crate::font_names::Source::Bundled => "Icons: bundled JetBrains Mono Nerd Font (fonts folder).",
                    crate::font_names::Source::Installed => "Icons: the JetBrains Mono Nerd Font installed in Windows (fonts folder missing).",
                    crate::font_names::Source::None => "No Nerd Font found: menus use Segoe UI with plain letters.",
                }));
                rows.push(info("Used by the menu, app picker, keybinds list and the main window. Icons always use the Nerd Font. The top bar font is under Appearance → Top bar → Bar items."));
                rows.push(info("Other fonts installed in Windows:"));
                rows.extend(all.map(row));
                rows
            }
            "custom-apps" => {
                let folder = self.profile_folder().to_path_buf();
                let mut rows = vec![
                    Row::new(PLUS, "Add an app, file or shortcut…", Act::Choose("app-browse", "new".into())),
                    Row::new(PLUS, "Add by typing a path (files or folders)", Act::Open(Level::Page("app-new"))),
                ];
                for (i, entry) in self.config.applications.iter().enumerate() {
                    let found = entry.target_path(&folder).exists();
                    rows.push(
                        Row::new(RUN, entry.name.clone(), Act::Choose("app-pick", i.to_string()))
                            .detail(if found { "available" } else { "missing on this PC" }),
                    );
                }
                if self.config.applications.is_empty() {
                    rows.push(info("No custom apps yet. They appear in the app picker beside installed apps."));
                }
                rows.push(info("Relative paths follow the active profile folder; full drive paths stay as written."));
                rows
            }
            "custom-app" => match self.custom_app.and_then(|i| self.config.applications.get(i)) {
                None => vec![info("That entry no longer exists.")],
                Some(e) => {
                    let path = e.target_path(self.profile_folder());
                    let none = |v: &str, empty: &str| if v.is_empty() { empty.to_string() } else { v.to_string() };
                    vec![
                        Row::new(EDIT, format!("Name: {}", e.name), Act::Open(Level::Page("app-name"))),
                        Row::new(FOLDER, format!("Path: {}", e.target), Act::Open(Level::Page("app-target")))
                            .detail(if path.exists() { "available" } else { "missing on this PC" }),
                        Row::new(FOLDER, "Choose a different file…", Act::Choose("app-browse", "change".into())),
                        Row::new(EDIT, format!("Arguments: {}", none(&e.arguments, "(none)")), Act::Open(Level::Page("app-args"))),
                        Row::new(EDIT, format!("Working folder: {}", none(&e.working_directory, "(the app's folder)")), Act::Open(Level::Page("app-folder"))),
                        Row::new(TRASH, "Delete entry", Act::Choose("app-remove", String::new())).detail("choose twice"),
                        info("Deleting an entry does not delete the program or its files."),
                    ]
                }
            },
            "app-new" | "app-name" | "app-target" | "app-args" | "app-folder" => return self.text_view(id),
            "bar-items" => {
                let mut rows = vec![info("Choose a section, then add, move or remove its modules. Changes apply at once; Undo changes puts the last one back.")];
                for (s, page) in BAR_SECTIONS.iter().enumerate() {
                    let titles: Vec<&str> = self.config.bar.section(s).iter().map(|m| m.title()).collect();
                    rows.push(
                        Row::new(GRID, page_title(page), Act::Open(Level::Page(page)))
                            .detail(if titles.is_empty() { "empty".to_string() } else { titles.join(" · ") }),
                    );
                }
                rows.push(info("Running apps and the open-window drawer are independent. Adding a module that is already on the bar moves it; dividers can repeat."));
                rows
            }
            "bar-left" | "bar-center" | "bar-right" => {
                let s = BAR_SECTIONS.iter().position(|p| *p == id).unwrap_or(0);
                let mut rows: Vec<Row> = self
                    .config
                    .bar
                    .section(s)
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        Row::new(GRID, m.title(), Act::Choose("bar-pick", format!("{s}:{i}"))).detail(m.description())
                    })
                    .collect();
                if rows.is_empty() {
                    rows.push(info("This section is empty."));
                }
                rows.push(Row::new(PLUS, "Add a module to this section", Act::Choose("bar-add-open", s.to_string())));
                rows
            }
            "bar-module" => {
                let (s, i) = self.bar_pick;
                match self.config.bar.section(s).get(i) {
                    None => vec![info("That module is no longer in this section.")],
                    Some(m) => {
                        let mut rows = vec![info(format!("{}: {}", m.title(), m.description()))];
                        if *m != crate::bar_model::Module::Divider {
                            let current = self.config.bar.display(*m);
                            for mode in crate::bar_model::DisplayMode::ALL {
                                rows.push(
                                    Row::new(OPTION, format!("Show as: {}", mode.label().to_lowercase()), Act::Choose("bar-display", mode.label().into()))
                                        .checked(current == mode),
                                );
                            }
                        }
                        rows.extend([
                            Row::new(SUBMENU, "Move up", Act::Choose("bar-move", "up".into())),
                            Row::new(SUBMENU, "Move down", Act::Choose("bar-move", "down".into())),
                        ]);
                        for (t, page) in BAR_SECTIONS.iter().enumerate() {
                            if t != s {
                                rows.push(Row::new(SUBMENU, format!("Move to the {}", page_title(page).to_lowercase()), Act::Choose("bar-move", t.to_string())));
                            }
                        }
                        rows.push(Row::new(TRASH, "Remove from the bar", Act::Choose("bar-move", "remove".into())));
                        rows
                    }
                }
            }
            "bar-add" => {
                let s = self.bar_pick.0;
                bar_editor::modules()
                    .into_iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let placed = (0..3).any(|t| self.config.bar.section(t).contains(&m));
                        Row::new(PLUS, m.title(), Act::Choose("bar-add", i.to_string()))
                            .checked(placed && m != crate::bar_model::Module::Divider && self.config.bar.section(s).contains(&m))
                            .detail(m.description())
                    })
                    .collect()
            }
            "workspace-layouts" => {
                let scope = self.layout_page_scope();
                let exact = scope.as_ref().and_then(|(d, m)| {
                    self.layout_preferences
                        .entries
                        .iter()
                        .find(|e| e.desktop == *d && e.monitor == *m)
                        .map(|e| e.layout.clone())
                });
                let desktop = if self.layout_all_desktops {
                    "All desktops".to_string()
                } else {
                    match &scope {
                        Some((d, _)) => format!("This desktop ({})", workspace_ui::desktop_label(d)),
                        None => "This desktop (Windows desktop information is unavailable)".into(),
                    }
                };
                let mut rows = vec![
                    info(format!("Default layout: {}. Pick where a choice applies, then pick its layout.", workspace_ui::layout_label(&self.config.layout))),
                    Row::new(DESKTOP, format!("Desktop: {desktop}"), Act::Choose("ws-desktop", String::new()))
                        .detail("This desktop / All desktops"),
                    Row::new(DESKTOP, format!("Display: {}", workspace_ui::monitor_label(&self.layout_monitor)), Act::Choose("ws-display", String::new()))
                        .detail("steps through the displays"),
                ];
                for (name, label) in crate::workspace_layouts::LAYOUTS {
                    rows.push(
                        Row::new(GRID, *label, Act::Choose("ws-layout", (*name).into()))
                            .checked(exact.as_deref() == Some(*name)),
                    );
                }
                rows.push(
                    Row::new(UNDO, "Use inherited layout here", Act::Choose("ws-layout", String::new()))
                        .checked(scope.is_some() && exact.is_none()),
                );
                if self.layout_preferences.entries.is_empty() {
                    rows.push(info("No saved choices yet: every desktop and display uses the default layout."));
                } else {
                    rows.push(info("Saved choices (choose one twice to remove it):"));
                    for (i, entry) in self.layout_preferences.entries.iter().enumerate() {
                        rows.push(Row::new(TRASH, workspace_ui::entry_label(entry), Act::Choose("ws-remove", i.to_string())));
                    }
                }
                rows.push(info("Choices stay on this PC. Specific desktop/display choices override broader ones. Native Windows desktops still switch every monitor together."));
                rows
            }
            "terminal-colors" => {
                let applied = appearance_effects::terminal_applied(&self.directory);
                vec![
                    Row::new(BRUSH, "Add the themed profile to Windows Terminal", Act::Choose("terminal-colors", "apply".into()))
                        .checked(applied)
                        .detail(format!("theme {}", self.config.theme)),
                    Row::new(WINDOW, "Open Windows Terminal with that profile", Act::Choose("terminal-colors", "open".into())),
                    Row::new(UNDO, "Remove the themed profile", Act::Choose("terminal-colors", "restore".into())),
                    info("Adds an \"Omarchy Overlay\" profile and color scheme as a Windows Terminal fragment. Your own Terminal settings are not edited."),
                    info("Run Add again after changing the theme. Remove puts back whatever was there before."),
                ]
            }
            "background" => {
                let exe_folder = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("wallpapers")));
                let mut folders = vec![self.profile_folder().join("wallpapers")];
                folders.extend(exe_folder);
                // Like Omarchy's background switcher: the current theme's own pictures.
                folders.extend(appearance_effects::backgrounds_dir(&self.config.theme));
                let current = self.config.appearance_wallpaper.clone();
                let mut rows: Vec<Row> = wallpapers(&folders)
                    .into_iter()
                    .map(|path| {
                        let name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        let chosen = !current.is_empty()
                            && Path::new(&current).file_name() == path.file_name();
                        Row::new(IMAGE, name, Act::Choose("background", path.to_string_lossy().into()))
                            .checked(chosen)
                    })
                    .collect();
                if rows.is_empty() {
                    rows.push(info("No pictures found. Put .jpg or .png files in the wallpapers folder next to OmarchyOverlay.exe."));
                }
                rows.push(
                    Row::new(UNDO, "Put back my original Windows background", Act::Choose("background", "restore".into()))
                        .detail(if self.directory.join("wallpaper-restore.json").exists() { "" } else { "nothing to restore" }),
                );
                rows.push(Row::new(FOLDER, "Open the wallpapers folder", Act::Choose("background", "folder".into())));
                rows
            }
            "gaps" => values(
                "gaps",
                SLIDERS,
                self.config.gap,
                &[0, 2, 4, 6, 8, 10, 12, 16, 20, 24, 32].map(|g| (g, format!("{g} pixels"))),
            ),
            "main-size" => values(
                "main-size",
                SLIDERS,
                (self.config.main_ratio * 100.).round() as i32,
                &[40, 45, 50, 55, 60, 65, 70].map(|p| (p, format!("Main window takes {p}%"))),
            ),
            "layout" => {
                let monitor = MonitorFromWindow(self.menu_target(), MONITOR_DEFAULTTONEAREST);
                let current = self.layout_for_monitor(monitor);
                let mut rows: Vec<Row> = commands::LAYOUTS
                    .iter()
                    .map(|(l, label)| {
                        Row::new(GRID, *label, Act::Choose("layout", (*l).into()))
                            .checked(current == *l)
                    })
                    .collect();
                rows.push(Row::new(DESKTOP, "Layouts by workspace", Act::Open(Level::Page("workspace-layouts"))));
                rows
            }
            "columns" => values(
                "columns",
                GRID,
                self.config.scrolling_columns,
                &[1, 2, 3, 4].map(|c| (c, format!("{c} column{}", if c == 1 { "" } else { "s" }))),
            ),
            "opacity" => values(
                "opacity",
                OPACITY,
                self.config.active_opacity,
                &[100, 98, 95, 90, 85, 80].map(|p| (p, format!("{p}%"))),
            ),
            "inactive-opacity" => values(
                "inactive-opacity",
                OPACITY,
                self.config.inactive_opacity,
                &[100, 95, 90, 85, 80, 75, 70].map(|p| (p, format!("{p}%"))),
            ),
            "materials" => {
                let mut rows = vec![
                    Row::new(SLIDERS, "App windows", Act::Open(Level::Page("app-materials")))
                        .detail(material_name(&self.config.window_backdrop)),
                    Row::new(SLIDERS, "Omarchy Overlay menus", Act::Open(Level::Page("menu-materials")))
                        .detail(material_name(&self.config.menu_backdrop)),
                    info("Omarchy Overlay's own settings windows always stay plain so their text stays readable."),
                    info("Only apps that allow it change. An app's own rule wins. Ignore apps are never touched."),
                ];
                if windows_personalization::current("EnableTransparency") == Some(0) {
                    rows.push(
                        Row::new(WARNING, "Windows transparency effects are off, so Mica and Acrylic look solid", Act::Open(Level::Page("windows-theme")))
                            .detail("Enter opens Apply theme to Windows"),
                    );
                }
                rows
            }
            "app-materials" | "menu-materials" => {
                let current = if id == "app-materials" {
                    &self.config.window_backdrop
                } else {
                    &self.config.menu_backdrop
                };
                let mut rows: Vec<Row> = crate::config::MATERIALS
                    .into_iter()
                    .map(|m| {
                        Row::new(SLIDERS, material_label(m), Act::Choose(id, m.into()))
                            .checked(current == m)
                    })
                    .collect();
                rows.push(info(if id == "app-materials" {
                    "Other apps' windows. An app's own rule wins; Ignore apps are never touched."
                } else {
                    "The Omarchy Overlay menu, the app picker and launcher. A tint keeps the text readable over any window."
                }));
                if windows_personalization::current("EnableTransparency") == Some(0) {
                    rows.push(
                        Row::new(WARNING, "Windows transparency effects are off, so Mica and Acrylic look solid", Act::Open(Level::Page("windows-theme")))
                            .detail("Enter opens Apply theme to Windows"),
                    );
                }
                rows
            }
            "animations" => {
                let mut rows = on_off("animations", self.config.animations, "Animate window moves", "No animation");
                for ms in [100, 140, 200, 300] {
                    rows.push(
                        Row::new(CLOCK, format!("{ms} ms"), Act::Choose("animations", ms.to_string()))
                            .checked(self.config.animation_duration_ms == ms),
                    );
                }
                rows
            }
            "focus-outline" => on_off("focus-outline", self.config.focus_outline, "Outline the active window", "No outline"),
            "osd" => on_off("osd", self.config.show_osd, "Show short messages on screen", "No on-screen messages"),
            "start-tiling" => {
                let mut rows = on_off(
                    "start-tiling",
                    self.config.start_tiling,
                    "Start with tiling on",
                    "Start paused",
                );
                rows.push(info("Used at sign-in, on Restart Omarchy Overlay and after an update. Pausing during a session does not change it. Window recovery always runs first."));
                rows
            }
            "bar" => {
                let mut rows = on_off("bar", self.config.show_top_bar, "Show the top bar", "Hide the top bar");
                rows.extend(self.menu_view("bar").rows.into_iter().skip(1));
                rows
            }
            "bar-style" => {
                use crate::bar_model::DisplayMode;
                let bar = &self.config.bar;
                let all = bar.all_display();
                let mut rows: Vec<Row> = [
                    (DisplayMode::Icon, "Icons", "Omarchy's look; the clock and window title stay text"),
                    (DisplayMode::IconText, "Icons and text", "an icon beside each name"),
                    (DisplayMode::Text, "Text", "names only, no icons"),
                ]
                .into_iter()
                .map(|(mode, label, detail)| {
                    Row::new(OPTION, label, Act::Choose("bar-style", mode.label().into()))
                        .checked(all == Some(mode))
                        .detail(detail)
                })
                .collect();
                if all.is_none() {
                    rows.push(info("Items are set one by one. Choose a style above to set them all."));
                }
                rows.push(info("Or set each item on the bar. Enter steps icon, icon and text, text."));
                let mut shown = vec![];
                for m in (0..3).flat_map(|s| bar.section(s).iter().copied()) {
                    if m != crate::bar_model::Module::Divider && !shown.contains(&m) {
                        shown.push(m);
                        rows.push(
                            Row::new(GRID, m.title(), Act::Choose("bar-style-item", m.name().into()))
                                .detail(bar.display(m).label()),
                        );
                    }
                }
                rows
            }
            "clock" => crate::bar_model::CLOCK_FORMATS
                .iter()
                .map(|(f, label)| {
                    Row::new(CLOCK, *label, Act::Choose("clock", (*f).into()))
                        .checked(self.config.bar.clock_format == *f)
                })
                .collect(),
            "bar-height" => values(
                "bar-height",
                SLIDERS,
                self.config.bar.height,
                &[24, 26, 28, 30, 34, 40].map(|h| (h, format!("{h} pixels"))),
            ),
            "bar-opacity" => values(
                "bar-opacity",
                OPACITY,
                self.config.bar_opacity,
                &[100, 95, 90, 85, 80, 70, 60, 50].map(|p| (p, format!("{p}%"))),
            ),
            "taskbar" => [
                ("legacy", "Follow the top bar (0.15 behavior)"),
                ("visible", "Always show the Windows taskbar"),
                ("auto-hide", "Auto-hide the Windows taskbar"),
                ("hidden", "Hide the Windows taskbar while Omarchy Overlay runs"),
            ]
            .into_iter()
            .map(|(m, label)| {
                Row::new(WINDOWS, label, Act::Choose("taskbar", m.into()))
                    .checked(self.config.taskbar_mode == m)
            })
            .collect(),
            "keymap" => crate::config::KEYMAPS
                .iter()
                .map(|(k, label)| {
                    Row::new(KEYBOARD, *label, Act::Choose("keymap", (*k).into()))
                        .checked(self.config.keymap == *k)
                })
                .collect(),
            "tap-super" => {
                let mut rows: Vec<Row> = crate::config::TAP_SUPER
                    .iter()
                    .map(|(k, label)| {
                        Row::new(KEYBOARD, *label, Act::Choose("tap-super", (*k).into()))
                            .checked(self.config.tap_super == *k)
                    })
                    .collect();
                rows.push(info("Releasing Super after a Omarchy Overlay key never opens Start."));
                rows
            }
            "keyboard" => self.keyboard_rows(),
            "conflicts" => self.conflict_rows(),
            "profiles" => {
                let mut rows: Vec<Row> = appearance_profiles::builtins()
                    .iter()
                    .chain(&self.config.appearance_profiles)
                    .enumerate()
                    .map(|(i, p)| {
                        let mine = i >= appearance_profiles::THEMES.len();
                        Row::new(if mine { STAR } else { BRUSH }, p.name.clone(), Act::Open(Level::Profile(i)))
                            .detail(if self.last_profile.as_deref() == Some(p.name.as_str()) {
                                "applied"
                            } else if mine {
                                "yours"
                            } else {
                                "built in"
                            })
                    })
                    .collect();
                rows.push(Row::new(SAVE, "Save current settings as a new profile", Act::Choose("profiles", "new".into())));
                rows.push(Row::new(FOLDER, "Profile files (import, export, portable, rename)", Act::Choose("profiles", "files".into())));
                rows
            }
            "app-rules" => {
                let mut rows = vec![
                    Row::new(COG, "Default for all other apps", Act::Open(Level::Rule(None)))
                        .detail(self.config.default_mode.label()),
                    Row::new(PLUS, "Add rule for an open window", Act::Open(Level::Page("add-rule"))),
                    Row::new(FLOAT, "Always-float list", Act::Open(Level::Page("always-float"))),
                ];
                for (i, rule) in self.config.rules.iter().enumerate() {
                    let mut label = crate::rules::app_name(&rule.executable).to_string();
                    if !rule.title_contains.is_empty() {
                        label = format!("{label}, title has \"{}\"", rule.title_contains);
                    }
                    if !rule.window_class.is_empty() {
                        label = format!("{label}, class {}", rule.window_class);
                    }
                    let mode = rule.mode.label();
                    rows.push(
                        Row::new(WINDOW, label, Act::Open(Level::Rule(Some(i))))
                            .detail(format!("{mode}  {}", rule.executable)),
                    );
                }
                rows
            }
            "add-rule" => {
                let mut rows: Vec<Row> = tasks::collect()
                    .iter()
                    .map(|t| {
                        let exe = process_name(t.hwnd);
                        let existing = self.rule_index(t.hwnd).is_some();
                        task_row(t, t.title.clone(), Act::Choose("add-rule", (t.hwnd as isize).to_string()))
                            .detail(format!("{exe}{}", if existing { "  (has a rule)" } else { "" }))
                    })
                    .collect();
                if rows.is_empty() {
                    rows.push(info("No open windows found."));
                }
                rows
            }
            "always-float" => {
                let list = &self.config.always_float;
                let mut rows = vec![info("Listed windows always float, whatever their app rule says.")];
                for (id, label) in crate::always_float::BUILTINS {
                    rows.push(
                        Row::new(FLOAT, *label, Act::Choose("always-float", (*id).into()))
                            .checked(list.enabled(id))
                            .detail("built in"),
                    );
                }
                for (i, entry) in list.windows.iter().enumerate() {
                    rows.push(
                        Row::new(WINDOW, entry.label(), Act::Choose("always-float-remove", i.to_string()))
                            .detail("Enter twice removes it"),
                    );
                }
                rows.push(Row::new(PLUS, "Add an open window", Act::Open(Level::Page("always-float-add"))));
                rows.push(Row::new(EDIT, "Edit always-float.json", Act::Choose("edit", "always-float.json".into())));
                rows
            }
            "always-float-add" => {
                let mut rows: Vec<Row> = tasks::collect()
                    .iter()
                    .map(|t| {
                        task_row(t, t.title.clone(), Act::Choose("always-float-add", (t.hwnd as isize).to_string()))
                            .detail(format!("{}  class {}", process_name(t.hwnd), window_class(t.hwnd)))
                    })
                    .collect();
                if rows.is_empty() {
                    rows.push(info("No open windows found."));
                }
                rows
            }
            "windows-theme" => self.windows_theme_rows(),
            "windows-checklist" => vec![
                info("Word, Excel, PowerPoint, Outlook: File → Account → Office Theme (Black or Dark Gray)."),
                info("Teams: Settings → Appearance and accessibility → Theme."),
                info("Chrome and Edge: follow the Windows dark mode once it is applied."),
                info("Windows that run as administrator: Omarchy Overlay cannot arrange or theme them."),
                info("Older programs with their own colors stay as they are."),
                Row::new(WINDOWS, "Windows color settings", Act::Run(Command::Settings("colors".into()))).windows(true),
            ],
            "webapps" => {
                let mut rows = vec![info("Removing only takes it out of Omarchy Overlay. Chrome keeps the app: remove it at chrome://apps.")];
                for app in &self.webapps {
                    rows.push(
                        Row::new(GLOBE, app.name.clone(), Act::Choose("webapps-remove", app.name.clone()))
                            .detail("Enter twice removes it"),
                    );
                }
                rows.push(Row::new(EDIT, "Edit webapps.json", Act::Choose("edit", "webapps.json".into())));
                rows.push(Row::new(DEFAULTS, "Reload config files", Act::Run(Command::Builtin("reload-config"))));
                rows
            }
            "install-webapp" => {
                let mut rows = vec![];
                for app in &self.webapps {
                    let installed = self.entries.iter().any(|e| e.label.eq_ignore_ascii_case(app.shortcut()));
                    rows.push(
                        Row::new(GLOBE, app.name.clone(), Act::Choose("install-webapp", app.name.clone()))
                            .checked(installed)
                            .detail(if installed {
                                "installed as a Chrome app"
                            } else {
                                "Enter opens the site in Chrome"
                            }),
                    );
                }
                rows.push(info("In Chrome: ⋮ menu → Cast, save and share → Install page as app. Omarchy Overlay never installs apps itself."));
                rows.push(Row::new(EDIT, "Add a web app: edit webapps.json", Act::Choose("edit", "webapps.json".into())));
                rows
            }
            "remove-apps" => {
                let mut rows = vec![info("Checked apps are hidden from the app picker. Nothing is uninstalled.")];
                for entry in self.entries.iter().filter(|e| e.action.is_none()) {
                    let key = entry.key();
                    let hidden = self.picker_history.hidden(&key);
                    rows.push(
                        Row::new(if hidden { CLOSE } else { RUN }, entry.label.clone(), Act::Choose("remove-apps", key))
                            .checked(hidden)
                            .detail(if hidden { "hidden" } else { "shown" }),
                    );
                }
                rows
            }
            "version" => {
                let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
                vec![
                    info(format!("{NAME} {}", env!("CARGO_PKG_VERSION"))),
                    info(format!("Program: {exe}")),
                    info(match profile::settings() {
                        Some(folder) => format!("Settings folder: {}", folder.display()),
                        None => "Settings folder: none (not using OVERLAY SETTINGS)".to_string(),
                    }),
                    info(format!("Profile: {}", self.profile.path.display())),
                    info(format!("Config folder: {}", config_folder::folder().display())),
                    Row::new(INFO, "Windows version and device name", Act::Run(Command::Settings("about".into()))).windows(true),
                    Row::new(COG, "Open the 0.15 settings window", Act::Choose("version", "dashboard".into())),
                    Row::new(INFO, "Omarchy Overlay guide", Act::Run(Command::Builtin("guide"))),
                    Row::new(FOLDER, "Omarchy Overlay releases", Act::Run(Command::Builtin("releases-folder"))),
                    Row::new(EDIT, "Omarchy Overlay log", Act::Run(Command::Builtin("log"))),
                ]
            }
            "config-errors" => {
                let mut rows: Vec<Row> = self
                    .config_errors
                    .iter()
                    .map(|e| Row::new(WARNING, e.clone(), Act::None))
                    .collect();
                if rows.is_empty() {
                    rows.push(info("Every config file loaded."));
                }
                rows.push(Row::new(FOLDER, "Open the config folder", Act::Run(Command::Builtin("config-folder"))));
                rows.push(Row::new(DEFAULTS, "Reload config files", Act::Run(Command::Builtin("reload-config"))));
                rows
            }
            "restore-failed" => {
                let mut rows = vec![];
                for &key in self.saved.keys() {
                    let h = key as HWND;
                    let name = if IsWindow(h) != 0 {
                        format!("{} ({})", text_of(h), process_name(h))
                    } else {
                        "A window that has since closed".into()
                    };
                    rows.push(Row::new(WINDOW, name, Act::Choose("restore-failed", key.to_string())).detail("Enter retries"));
                }
                if rows.is_empty() {
                    rows.push(info("Every window is back in its original place."));
                } else {
                    rows.insert(0, info("These windows could not be put back. Retry, or move them by hand."));
                    rows.push(Row::new(UNDO, "Retry all", Act::Choose("restore-failed", "all".into())));
                }
                rows
            }
            _ => vec![info("This page is not available.")],
        };
        if has_defaults(id) {
            rows.push(Row::new(UNDO, "Undo changes", Act::Undo));
            rows.push(Row::new(DEFAULTS, "Restore defaults", Act::Defaults));
        }
        let title = match id {
            "restore-failed" => "Windows not restored",
            _ => page_title(id),
        };
        page(title, rows)
    }
    fn keyboard_rows(&self) -> Vec<Row> {
        let state = if self.keys_paused {
            "Paused: every key belongs to Windows".to_string()
        } else if keyhook::running() {
            format!(
                "Super keys active: {} binds from binds.conf",
                self.binds.len()
            )
        } else if self.config.keymap == "classic" {
            "Super keys off (classic keymap)".into()
        } else {
            "Super keys unavailable: see Keyboard conflicts".into()
        };
        let label = |list: &[(&str, &str)], value: &str| {
            list.iter()
                .find(|(k, _)| *k == value)
                .map_or(String::new(), |(_, l)| (*l).to_string())
        };
        let mut rows = vec![
            info(state),
            Row::new(KEYBOARD, "Show all keybinds", Act::Open(Level::Keybinds)),
            Row::new(KEYBOARD, "Keymap", Act::Open(Level::Page("keymap")))
                .detail(label(crate::config::KEYMAPS, &self.config.keymap)),
            Row::new(
                KEYBOARD,
                "Tapping Super on its own",
                Act::Open(Level::Page("tap-super")),
            )
            .detail(label(crate::config::TAP_SUPER, &self.config.tap_super)),
        ];
        for (m, text) in [
            ("CtrlAlt", "Classic keys use Ctrl+Alt"),
            ("WinAlt", "Classic keys use Win+Alt"),
            ("Alt", "Classic keys use Alt alone"),
        ] {
            let mut row = Row::new(OPTION, text, Act::Choose("modifiers", m.into()))
                .checked(self.config.modifiers == m);
            if m == "Alt" {
                row = row.detail("takes Alt+letter shortcuts from programs");
            }
            rows.push(row);
        }
        let conflicts = self.conflict_count();
        rows.push(
            Row::new(
                if conflicts > 0 { WARNING } else { KEYBOARD },
                "Keyboard conflicts",
                Act::Open(Level::Page("conflicts")),
            )
            .detail(if conflicts > 0 {
                format!("{conflicts} taken")
            } else {
                "none".into()
            }),
        );
        rows.push(Row::new(
            EDIT,
            "Edit binds.conf",
            Act::Choose("edit", "binds.conf".into()),
        ));
        rows.push(
            Row::new(
                KEYBOARD,
                if self.keys_paused {
                    "Resume Omarchy Overlay keys"
                } else {
                    "Pause Omarchy Overlay keys"
                },
                Act::Run(Command::Builtin("pause-keys")),
            )
            .checked(self.keys_paused),
        );
        rows.push(info("Keys never reach windows that run as administrator. Windows blocks that for normal programs."));
        rows.push(info("Super+L and Ctrl+Alt+Del always stay with Windows."));
        rows
    }
    /// Classic actions whose chord another program or Windows already holds.
    fn conflicted(&self) -> Vec<(usize, String)> {
        if self.keys_paused || self.config.keymap == "omarchy" {
            return vec![];
        }
        crate::shortcuts::ACTIONS
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                let chord = crate::shortcuts::chord(
                    i,
                    &self.config.modifiers,
                    &self.config.shortcut_overrides,
                );
                crate::shortcuts::parse(&chord).ok().flatten()?;
                let id = BASE as i32 + crate::shortcuts::action_slot(i) as i32;
                (!self.hotkeys.contains(&id)).then_some((i, chord))
            })
            .collect()
    }
    fn conflict_count(&self) -> usize {
        self.conflicted().len()
            + self
                .failures
                .iter()
                .filter(|f| !f.starts_with("Shortcut unavailable"))
                .count()
    }
    /// Free chords for an action: Windows is asked for each one and released at once.
    unsafe fn alternatives(&self, index: usize, chord: &str) -> Vec<String> {
        let key = chord.rsplit('+').next().unwrap_or_default();
        let mut free = vec![];
        for prefix in [
            "Ctrl+Alt+Shift+",
            "Win+Alt+",
            "Win+Ctrl+Alt+",
            "Ctrl+Shift+",
        ] {
            let candidate = format!("{prefix}{key}");
            if candidate.eq_ignore_ascii_case(chord) {
                continue;
            }
            let Some(binding) = crate::shortcuts::parse(&candidate).ok().flatten() else {
                continue;
            };
            if crate::shortcuts::conflict(binding, index, &self.config).is_some() {
                continue;
            }
            let probe = BASE as i32 + 513;
            if RegisterHotKey(
                self.main,
                probe,
                binding.modifiers | MOD_NOREPEAT,
                binding.key,
            ) != 0
            {
                UnregisterHotKey(self.main, probe);
                free.push(candidate);
                if free.len() == 2 {
                    break;
                }
            }
        }
        free
    }
    unsafe fn conflict_rows(&self) -> Vec<Row> {
        let mut rows = vec![];
        for (i, chord) in self.conflicted() {
            let action = &crate::shortcuts::ACTIONS[i];
            rows.push(Row::new(
                WARNING,
                format!("{}: {chord} is taken", action.label),
                Act::None,
            ));
            for alternative in self.alternatives(i, &chord) {
                rows.push(
                    Row::new(
                        SUBMENU,
                        format!("Use {alternative} instead"),
                        Act::Choose("conflict", format!("{i}={alternative}")),
                    )
                    .detail("free"),
                );
            }
        }
        for failure in self
            .failures
            .iter()
            .filter(|f| !f.starts_with("Shortcut unavailable"))
        {
            rows.push(Row::new(WARNING, failure.clone(), Act::None));
        }
        for bind in &self.binds {
            if let Some(windows) = keyhook::replaces(bind.key) {
                rows.push(info(format!(
                    "{}: Omarchy Overlay uses it for \"{}\" instead of Windows {windows}",
                    bind.key.label(),
                    bind.description
                )));
            }
        }
        for holder in keyhook::holders() {
            rows.push(Row::new(
                WARNING,
                format!("{holder} is running and may hold keys"),
                Act::None,
            ));
        }
        if rows.is_empty() {
            rows.push(info("No conflicts: every Omarchy Overlay key is working."));
        }
        rows
    }
    unsafe fn windows_theme_rows(&self) -> Vec<Row> {
        let palette = theme::current();
        let light = windows_personalization::current("AppsUseLightTheme");
        let effect = |name: &str| windows_personalization::current(name) == Some(1);
        let state = |name: &str| match windows_personalization::current(name) {
            Some(1) => "now on",
            Some(_) => "now off",
            None => "",
        };
        vec![
            Row::new(
                if palette.dark { MOON } else { SUN },
                format!("Windows {} mode, to match the theme", if palette.dark { "dark" } else { "light" }),
                Act::Choose("windows-theme", "mode".into()),
            )
            .checked(light == Some(u32::from(!palette.dark)))
            .detail(match light {
                Some(0) => "now dark",
                Some(_) => "now light",
                None => "",
            }),
            Row::new(SLIDERS, "Transparency effects", Act::Choose("windows-theme", "EnableTransparency".into()))
                .checked(effect("EnableTransparency"))
                .detail(state("EnableTransparency")),
            Row::new(SLIDERS, "Accent color on Start and taskbar", Act::Choose("windows-theme", "ColorPrevalence".into()))
                .checked(effect("ColorPrevalence"))
                .detail(state("ColorPrevalence")),
            Row::new(BRUSH, "Accent color", Act::Run(Command::Settings("colors".into())))
                .windows(true)
                .detail(format!("Custom color {}", hex(theme::windows_accent(&self.config.theme, palette)))),
            Row::new(WINDOW, "Other apps' window frames follow the theme", Act::Choose("frames", String::new()))
                .checked(self.config.coordinate_windows)
                .detail("dark frame; App rules can override one app"),
            Row::new(SLIDERS, "Blur and materials (app windows, Omarchy Overlay menus)", Act::Open(Level::Page("materials"))),
            Row::new(IMAGE, "Background", Act::Open(Level::Page("background"))),
            Row::new(BRUSH, "Windows Terminal colors", Act::Run(Command::Page("terminal-colors"))),
            Row::new(UNDO, "Restore the original Windows appearance", Act::Run(Command::Builtin("restore-windows-appearance"))),
            Row::new(INFO, "Apps Omarchy Overlay cannot theme", Act::Open(Level::Page("windows-checklist"))),
            info("Omarchy Overlay records the original values first and puts them back on Restore, on exit and after a crash."),
        ]
    }
    unsafe fn rule_index(&self, h: HWND) -> Option<usize> {
        let rule = crate::rules::matching(
            &self.config.rules,
            &process_name(h),
            &text_of(h),
            &window_class(h),
        )?;
        self.config.rules.iter().position(|r| std::ptr::eq(r, rule))
    }
    unsafe fn rule_view(&self, index: Option<usize>) -> View {
        let Some(i) = index else {
            let mut rows = vec![info("Applies to every app without its own rule.")];
            rows.extend(mode_rows("default-mode", self.config.default_mode));
            rows.push(
                Row::new(
                    OPACITY,
                    "Allow opacity changes",
                    Act::Choose("default-opacity", String::new()),
                )
                .checked(self.config.default_allow_opacity),
            );
            rows.push(Row::new(UNDO, "Undo changes", Act::Undo));
            rows.push(Row::new(DEFAULTS, "Restore defaults", Act::Defaults));
            return page("Default for all other apps", rows);
        };
        let Some(rule) = self.config.rules.get(i) else {
            return page(
                "App rule",
                vec![info("This rule no longer exists. Esc goes back.")],
            );
        };
        let mut rows = vec![];
        let exe = rule.executable.to_ascii_lowercase();
        if exe == "outlook.exe" || exe == "olk.exe" {
            let other = if exe == "olk.exe" {
                "OUTLOOK.EXE"
            } else {
                "olk.exe"
            };
            rows.push(info(format!(
                "Classic Outlook is OUTLOOK.EXE and new Outlook is olk.exe. This rule covers {} only.",
                rule.executable
            )));
            if !self
                .config
                .rules
                .iter()
                .any(|r| r.executable.eq_ignore_ascii_case(other))
            {
                rows.push(Row::new(
                    PLUS,
                    format!("Add the same rule for {other}"),
                    Act::Choose("rule", "pair".into()),
                ));
            }
        }
        rows.extend(mode_rows("rule-mode", rule.mode));
        rows.push(
            Row::new(
                OPACITY,
                "Active opacity",
                Act::Choose("rule-active", String::new()),
            )
            .detail(percent(rule.active_opacity)),
        );
        rows.push(
            Row::new(
                OPACITY,
                "Inactive opacity",
                Act::Choose("rule-inactive", String::new()),
            )
            .detail(percent(rule.inactive_opacity)),
        );
        rows.push(
            Row::new(
                SLIDERS,
                "Material",
                Act::Choose("rule-material", String::new()),
            )
            .detail(rule.backdrop.clone().unwrap_or("app default".into())),
        );
        rows.push(
            Row::new(
                BRUSH,
                "Match the Omarchy Overlay theme",
                Act::Choose("rule-theme", String::new()),
            )
            .detail(match rule.coordinate_theme {
                None => "app default",
                Some(true) => "on",
                Some(false) => "off",
            }),
        );
        rows.push(
            Row::new(
                OPACITY,
                "Allow opacity changes",
                Act::Choose("rule-opacity", String::new()),
            )
            .checked(rule.allow_opacity),
        );
        rows.push(
            Row::new(
                TRASH,
                "Delete this rule",
                Act::Choose("rule", "delete".into()),
            )
            .detail("Enter twice"),
        );
        rows.push(Row::new(UNDO, "Undo changes", Act::Undo));
        rows.push(Row::new(DEFAULTS, "Restore defaults", Act::Defaults));
        let windows: Vec<tasks::Task> = tasks::collect()
            .into_iter()
            .filter(|t| process_name(t.hwnd).eq_ignore_ascii_case(&rule.executable))
            .collect();
        if !windows.is_empty() {
            rows.push(info("Open windows of this program now:"));
        }
        for t in &windows {
            rows.push(
                task_row(t, t.title.clone(), Act::None).detail(self.window_behavior_reason(t.hwnd)),
            );
        }
        rows.push(info("A window not following it may have a different program name, a manual float, a protected dialog or an Ignore rule."));
        page(crate::rules::app_name(&rule.executable), rows)
    }
    unsafe fn window_panel_view(&self, h: HWND) -> View {
        if h.is_null() || IsWindow(h) == 0 || h == self.main {
            return page(
                "Selected window",
                vec![info("No window was in front when the menu opened. Click a window, then open the menu again.")],
            );
        }
        let exe = process_name(h);
        let app = crate::rules::app_name(&exe);
        let mode = window_policy(&self.config, h).mode;
        let mut rows = vec![info(format!("{app}: {}", self.window_behavior_reason(h)))];
        for mut row in mode_rows("panel-mode", mode) {
            row.label = format!("This app: {}", row.label);
            rows.push(row);
        }
        rows.push(
            Row::new(
                FLOAT,
                "Float or tile this window only",
                Act::Choose("panel", "float".into()),
            )
            .checked(self.floating.contains(&(h as isize))),
        );
        for p in [100, 95, 90, 85] {
            rows.push(Row::new(
                OPACITY,
                format!("Opacity {p}%"),
                Act::Choose("panel-opacity", p.to_string()),
            ));
        }
        for (value, icon, label) in [
            ("maximize", WINDOW, "Maximize or restore"),
            ("main", GRID, "Make it the main window"),
            ("monitor", DESKTOP, "Move to the next monitor"),
            ("scratchpad", STAR, "Use as scratchpad"),
            ("rule", COG, "Full rule for this app"),
            ("always-float", FLOAT, "Always float this kind of window"),
            ("close", CLOSE, "Close the window"),
        ] {
            rows.push(Row::new(icon, label, Act::Choose("panel", value.into())));
        }
        page(&format!("{app}: {}", text_of(h)), rows)
    }
    fn profile_list(&self) -> Vec<Profile> {
        appearance_profiles::builtins()
            .into_iter()
            .chain(self.config.appearance_profiles.iter().cloned())
            .collect()
    }
    fn profile_view(&self, i: usize) -> View {
        let Some(profile) = self.profile_list().get(i).cloned() else {
            return page(
                "Saved profile",
                vec![info("This profile no longer exists. Esc goes back.")],
            );
        };
        let mut after = self.config.clone();
        let mut rows = vec![];
        if let Err(error) = profile.apply(&mut after) {
            rows.push(Row::new(WARNING, error, Act::None));
        }
        let c = &self.config;
        let yes = |b: bool| if b { "on" } else { "off" }.to_string();
        let changes = [
            ("Theme", c.theme.clone(), after.theme.clone()),
            ("Gaps", c.gap.to_string(), after.gap.to_string()),
            (
                "Bar font",
                c.bar.font_family.clone(),
                after.bar.font_family.clone(),
            ),
            (
                "Bar text size",
                c.bar.font_size.to_string(),
                after.bar.font_size.to_string(),
            ),
            (
                "Bar height",
                c.bar.height.to_string(),
                after.bar.height.to_string(),
            ),
            (
                "Bar opacity",
                format!("{}%", c.bar_opacity),
                format!("{}%", after.bar_opacity),
            ),
            (
                "Focus outline",
                yes(c.focus_outline),
                yes(after.focus_outline),
            ),
            ("On-screen messages", yes(c.show_osd), yes(after.show_osd)),
            (
                "Background",
                c.appearance_wallpaper.clone(),
                after.appearance_wallpaper.clone(),
            ),
            (
                "Window opacity",
                format!("{}%", c.active_opacity),
                format!("{}%", after.active_opacity),
            ),
            (
                "Inactive opacity",
                format!("{}%", c.inactive_opacity),
                format!("{}%", after.inactive_opacity),
            ),
            (
                "Material for app windows",
                c.window_backdrop.clone(),
                after.window_backdrop.clone(),
            ),
            (
                "Material for Omarchy Overlay menus",
                c.menu_backdrop.clone(),
                after.menu_backdrop.clone(),
            ),
            (
                "Other apps' window frames",
                yes(c.coordinate_windows),
                yes(after.coordinate_windows),
            ),
            ("Animations", yes(c.animations), yes(after.animations)),
        ];
        for (name, old, new) in changes {
            if old != new {
                let shown = |s: String| if s.is_empty() { "none".to_string() } else { s };
                rows.push(Row::new(
                    SUBMENU,
                    format!("{name}: {} → {}", shown(old), shown(new)),
                    Act::None,
                ));
            }
        }
        if rows.is_empty() {
            rows.push(info(
                "Nothing changes: this profile matches the current settings.",
            ));
        } else {
            rows.insert(0, info("Applying this profile changes:"));
        }
        rows.push(Row::new(
            RUN,
            "Apply this profile",
            Act::Choose("profile", "apply".into()),
        ));
        if i >= appearance_profiles::THEMES.len() {
            rows.push(Row::new(
                SAVE,
                "Update this profile from current settings",
                Act::Choose("profile", "update".into()),
            ));
            rows.push(
                Row::new(
                    TRASH,
                    "Delete this profile",
                    Act::Choose("profile", "delete".into()),
                )
                .detail("Enter twice"),
            );
        }
        rows.push(info("Windows dark mode, accent and transparency stay as they are. Restore puts back the Windows originals."));
        page(&profile.name, rows)
    }

    /// Saves a changed configuration and applies only what changed.
    pub(super) unsafe fn commit(&mut self, config: Config) -> bool {
        if let Err(error) = config
            .validate()
            .and_then(|()| config.always_float.validate())
        {
            overlay::message(&error);
            return false;
        }
        let old = std::mem::replace(&mut self.config, config);
        let c = &self.config;
        let theme_changed = old.theme != c.theme;
        let bar_changed = differs(&old.bar, &c.bar);
        let taskbar_changed = old.show_top_bar != c.show_top_bar
            || old.auto_hide_taskbar != c.auto_hide_taskbar
            || old.taskbar_mode != c.taskbar_mode
            || old.bar_opacity != c.bar_opacity;
        let keys_changed = old.keymap != c.keymap
            || old.tap_super != c.tap_super
            || old.modifiers != c.modifiers
            || old.shortcut_overrides != c.shortcut_overrides;
        let font_changed = old.menu_font != c.menu_font || old.menu_font_size != c.menu_font_size;
        let float_changed = old.always_float != c.always_float;
        let layout_changed = old.gap != c.gap
            || old.main_ratio != c.main_ratio
            || old.layout != c.layout
            || old.scrolling_columns != c.scrolling_columns;
        if theme_changed {
            let name = self.config.theme.clone();
            self.apply_theme(&name);
        }
        if bar_changed {
            bar::settings(&self.config.bar);
        }
        if taskbar_changed {
            self.taskbar_hidden_preferred = self.config.taskbar_mode == "hidden";
            bar::configure(
                self.config.show_top_bar,
                self.config.auto_hide_taskbar,
                &self.config.taskbar_mode,
                self.config.bar_opacity,
            );
            bar::appearance(self.config.bar_opacity);
        }
        if keys_changed {
            self.restart_keys();
        }
        if font_changed {
            overlay::set_font(&self.config.menu_font, self.config.menu_font_size);
        }
        if old.animations && !self.config.animations {
            self.finish_motion(true);
        }
        if float_changed {
            match serde_json::to_string_pretty(&self.config.always_float) {
                Ok(text) => {
                    if let Err(error) = config_folder::save("always-float.json", &text) {
                        overlay::message(&error);
                    }
                }
                Err(error) => log(&error.to_string()),
            }
        }
        if layout_changed {
            self.dirty = true;
        }
        self.apply_own_appearance();
        self.apply_rules();
        if self.active {
            self.tick();
        }
        self.update_dashboard();
        self.update_appearance();
        true
    }
    /// Keeps the settings from before the first change on a page.
    fn remember(&mut self, page: &'static str) {
        if self.overlay_undo.as_ref().is_some_and(|u| u.page == page) {
            return;
        }
        self.overlay_undo = Some(Undo {
            page,
            config: self.config.clone(),
            webapps: self.webapps.clone(),
            picker: self.picker_history.clone(),
        });
    }
    fn save_webapps(&self) {
        if let Err(error) =
            config_folder::save("webapps.json", &crate::webapps::to_text(&self.webapps))
        {
            unsafe { overlay::message(&error) };
        }
    }
    /// Destructive rows need a second Enter; the first one only says so.
    unsafe fn twice(&mut self, page: &'static str, value: &str, what: &str) -> bool {
        if self
            .confirm_choice
            .as_ref()
            .is_some_and(|(p, v)| *p == page && v == value)
        {
            self.confirm_choice = None;
            return true;
        }
        self.confirm_choice = Some((page, value.into()));
        overlay::message(&format!("Press Enter again to {what}."));
        false
    }
    /// Hides the menu and runs the command once the window behind it is in front again.
    unsafe fn run_after_menu(&mut self, command: Command, confirmed: bool) {
        let target = overlay::target();
        overlay::hide();
        if !target.is_null() && IsWindow(target) != 0 {
            SetForegroundWindow(target);
        }
        self.pending_command = Some((command, confirmed));
        PostMessageW(self.main, command_run::PENDING_COMMAND, 0, 0);
    }

    /// Everything the menu asks App to do.
    pub(super) unsafe fn overlay_act(&mut self, act: Act) {
        let repeated = match (&act, &self.confirm_choice) {
            (Act::Choose(p, v), Some((cp, cv))) => p == cp && v == cv,
            (Act::Defaults, Some((cp, _))) => *cp == "defaults",
            _ => false,
        };
        if !repeated {
            self.confirm_choice = None;
        }
        match act {
            Act::None => {}
            Act::Run(command) => self.overlay_run(command),
            Act::Confirm(command, question) => {
                self.push(Level::Confirm(Box::new(command), question))
            }
            Act::Open(level) => self.push(level),
            Act::Choose(page, value) => {
                if self.choose(page, &value) && overlay::visible() {
                    self.refresh_view();
                }
            }
            Act::Launch(i) => {
                overlay::hide();
                self.launch_entry(i);
            }
            Act::Window(h) => {
                overlay::hide();
                if let Some(task) = tasks::collect().into_iter().find(|t| t.hwnd as isize == h) {
                    tasks::activate(&task, false);
                }
            }
            Act::Back => {
                if let Some(level) = overlay::back() {
                    let view = self.view(&level);
                    overlay::returned(view);
                }
            }
            Act::Undo => {
                self.undo();
                self.refresh_view();
            }
            Act::Defaults => {
                let key = overlay::level().as_ref().map_or("", level_key);
                if self.twice("defaults", key, "restore the defaults on this page") {
                    self.restore_defaults(key);
                    self.refresh_view();
                }
            }
        }
    }
    unsafe fn overlay_run(&mut self, command: Command) {
        match &command {
            Command::Menu(id) => self.push(Level::Menu(id.clone())),
            Command::Page(p) => self.push(Level::Page(p)),
            Command::Builtin("menu") => self.push(Level::Menu("main".into())),
            Command::Builtin("apps") => self.push(Level::Apps),
            Command::Builtin("keybinds") => self.push(Level::Keybinds),
            Command::Builtin("open-windows") => self.push(Level::Windows),
            Command::Builtin("window-panel") => {
                self.push(Level::WindowPanel(overlay::target() as isize))
            }
            _ if command_run::question(&command).is_some() => {
                let question = command_run::question(&command).unwrap_or_default();
                self.push(Level::Confirm(Box::new(command), question.into()));
            }
            Command::Theme(name) => {
                self.remember("theme");
                let mut c = self.config.clone();
                c.theme = name.clone();
                self.commit(c);
                self.refresh_view();
            }
            _ if self.command_state(&command).is_some() => {
                self.run_command(&command, false);
                self.refresh_view();
            }
            _ => self.run_after_menu(command, false),
        }
    }
    unsafe fn undo(&mut self) {
        let key = overlay::level().as_ref().map_or("", level_key);
        match self.overlay_undo.take() {
            Some(undo) if undo.page == key => {
                match key {
                    "background" => {
                        if let Err(error) = appearance_effects::with_effects(|| appearance_effects::restore_wallpaper(&self.directory)) {
                            overlay::message(&error);
                        }
                    }
                    "windows-theme" => {
                        if let Err(error) = appearance_effects::with_effects(|| windows_personalization::restore(&self.directory)) {
                            overlay::message(&error);
                        }
                    }
                    "webapps" | "install-webapp" => {
                        self.webapps = undo.webapps;
                        self.save_webapps();
                    }
                    "remove-apps" => {
                        self.picker_history = undo.picker;
                        self.save_picker();
                        self.filter_apps();
                    }
                    _ => {}
                }
                if self.commit(undo.config) {
                    overlay::message("Changes on this page were undone.");
                }
            }
            other => {
                self.overlay_undo = other;
                overlay::message("Nothing to undo on this page yet.");
            }
        }
    }
    unsafe fn restore_defaults(&mut self, key: &'static str) {
        self.remember(key);
        let mut c = self.config.clone();
        if page_defaults(key, &mut c) {
            if key == "taskbar" {
                self.set_taskbar_policy(&c.taskbar_mode.clone());
            }
            if self.commit(c) {
                overlay::message("Defaults restored on this page.");
            }
            return;
        }
        let result = match key {
            "background" => {
                c.appearance_wallpaper.clear();
                self.commit(c);
                appearance_effects::with_effects(|| appearance_effects::restore_wallpaper(&self.directory))
            }
            "windows-theme" => appearance_effects::with_effects(|| windows_personalization::restore(&self.directory)),
            "webapps" | "install-webapp" => {
                self.webapps = crate::webapps::defaults();
                self.save_webapps();
                Ok(())
            }
            "remove-apps" => {
                let keys: Vec<String> = self.entries.iter().map(|e| e.key()).collect();
                for k in keys {
                    self.picker_history.set_hidden(&k, false);
                }
                self.save_picker();
                self.filter_apps();
                Ok(())
            }
            "rule" => {
                let Some(Level::Rule(Some(i))) = overlay::level() else {
                    return;
                };
                let Some(old) = self.config.rules.get(i) else {
                    return;
                };
                let rule = AppRule {
                    executable: old.executable.clone(),
                    title_contains: old.title_contains.clone(),
                    window_class: old.window_class.clone(),
                    ..Default::default()
                };
                self.save_app_rule(Some(i), rule);
                Ok(())
            }
            _ => Err("This page has no defaults.".into()),
        };
        overlay::message(&match result {
            Ok(()) => "Defaults restored on this page.".into(),
            Err(error) => error,
        });
    }
    /// A choice on a page. Returns true when the page should be redrawn.
    unsafe fn choose(&mut self, page: &'static str, value: &str) -> bool {
        let key = overlay::level().as_ref().map_or("", level_key);
        let mut c = self.config.clone();
        let number = value.parse::<i32>().unwrap_or(0);
        let index = value.parse::<usize>().unwrap_or(0);
        match page {
            "theme" => c.theme = value.into(),
            "font" => c.menu_font = value.into(),
            "font-size" => c.menu_font_size = number,
            "gaps" => c.gap = [0, 2, 4, 6, 8, 10, 12, 16, 20, 24, 32][index.min(10)],
            "main-size" => {
                c.main_ratio = f64::from([40, 45, 50, 55, 60, 65, 70][index.min(6)]) / 100.
            }
            "columns" => c.scrolling_columns = index + 1,
            "opacity" => c.active_opacity = [100, 98, 95, 90, 85, 80][index.min(5)],
            "inactive-opacity" => c.inactive_opacity = [100, 95, 90, 85, 80, 75, 70][index.min(6)],
            "bar-height" => {
                c.bar.height = [24, 26, 28, 30, 34, 40][index.min(5)];
                c.bar.font_size = c.bar.font_size.min(c.bar.height - 8);
            }
            "bar-opacity" => c.bar_opacity = [100, 95, 90, 85, 80, 70, 60, 50][index.min(7)],
            "materials" | "app-materials" => c.window_backdrop = value.into(),
            "menu-materials" => c.menu_backdrop = value.into(),
            "animations" if value == "on" || value == "off" => c.animations = value == "on",
            "animations" => {
                c.animations = true;
                c.animation_duration_ms = number as u32;
            }
            "focus-outline" => c.focus_outline = value == "on",
            "osd" => c.show_osd = value == "on",
            "start-tiling" => c.start_tiling = value == "on",
            "bar" => c.show_top_bar = value == "on",
            "clock" => c.bar.clock_format = value.into(),
            "bar-style" => {
                let Some(mode) = display_mode(value) else {
                    return false;
                };
                c.bar.set_all_display(mode);
            }
            "bar-style-item" => {
                let Some(module) = crate::bar_model::Module::from_key(value) else {
                    return false;
                };
                c.bar.cycle_display(module);
            }
            "bar-display" => {
                let (s, i) = self.bar_pick;
                let (Some(&module), Some(mode)) = (self.config.bar.section(s).get(i), display_mode(value)) else {
                    return false;
                };
                c.bar.module_display.insert(module, mode);
            }
            "keymap" => c.keymap = value.into(),
            "tap-super" => c.tap_super = value.into(),
            "modifiers" => c.modifiers = value.into(),
            "default-mode" => c.default_mode = mode_of(value),
            "default-opacity" => c.default_allow_opacity = !c.default_allow_opacity,
            "frames" => c.coordinate_windows = !c.coordinate_windows,
            "conflict" => {
                let Some((i, chord)) = value.split_once('=') else {
                    return false;
                };
                let id = crate::shortcuts::ACTIONS[i.parse::<usize>().unwrap_or(0)].id;
                c.shortcut_overrides.insert(id.into(), chord.into());
            }
            "always-float-add" => {
                let h = value.parse::<isize>().unwrap_or(0) as HWND;
                let exe = process_name(h);
                if IsWindow(h) == 0 || !exe.to_ascii_lowercase().ends_with(".exe") {
                    overlay::message("Omarchy Overlay cannot read this window's program name. It may run as administrator.");
                    return false;
                }
                c.always_float.windows.push(crate::always_float::Entry {
                    executable: exe,
                    window_class: window_class(h),
                    title_contains: String::new(),
                });
                self.remember("always-float");
                if self.commit(c) {
                    if let Some(level) = overlay::back() {
                        let view = self.view(&level);
                        overlay::returned(view);
                    }
                    overlay::message("Added. It floats from now on.");
                }
                return false;
            }
            "always-float" => {
                if c.always_float.enabled(value) {
                    // Switching a built-in off lets those dialogs tile, so it asks first.
                    if !self.twice(
                        page,
                        value,
                        "switch it off: these windows then follow app rules and can tile",
                    ) {
                        return false;
                    }
                    c.always_float.disabled.push(value.into());
                } else {
                    c.always_float.disabled.retain(|d| d != value);
                }
            }
            "always-float-remove" => {
                if index >= c.always_float.windows.len() || !self.twice(page, value, "remove it") {
                    return false;
                }
                c.always_float.windows.remove(index);
            }
            _ => return self.choose_action(page, value, key),
        }
        self.remember(key);
        if self.commit(c) {
            overlay::message("Saved.");
        }
        true
    }
    /// Choices that act instead of changing one setting.
    unsafe fn choose_action(&mut self, page: &'static str, value: &str, key: &'static str) -> bool {
        match page {
            "confirm" => {
                if let Some(Level::Confirm(command, _)) = overlay::level() {
                    self.run_after_menu(*command, true);
                }
                false
            }
            "edit" => {
                self.edit_config_file(value);
                false
            }
            "layout" => {
                self.remember(key);
                self.set_layout(value);
                true
            }
            "taskbar" => {
                self.remember(key);
                self.set_taskbar_policy(value);
                true
            }
            "background" => {
                let result = match value {
                    "folder" => {
                        let folder = self.profile_folder().join("wallpapers");
                        let _ = fs::create_dir_all(&folder);
                        shell(&folder, "");
                        Ok(())
                    }
                    "restore" => {
                        self.remember(key);
                        let mut c = self.config.clone();
                        c.appearance_wallpaper.clear();
                        self.commit(c);
                        appearance_effects::with_effects(|| appearance_effects::restore_wallpaper(&self.directory))
                    }
                    path => {
                        self.remember(key);
                        let mut path = PathBuf::from(path);
                        // 0.18: a picture from the version folder is copied into the settings
                        // folder first, so the profile never names a version folder or drive.
                        if profile::settings().is_some() && !path.starts_with(self.profile_folder()) {
                            if let Some(name) = path.file_name() {
                                let copy = self.profile_folder().join("wallpapers").join(name);
                                let _ = fs::create_dir_all(self.profile_folder().join("wallpapers"));
                                if copy.exists() || fs::copy(&path, &copy).is_ok() {
                                    path = copy;
                                }
                            }
                        }
                        appearance_effects::with_effects(|| appearance_effects::apply_wallpaper(&self.directory, &path)).map(|()| {
                            let mut c = self.config.clone();
                            c.appearance_wallpaper = path
                                .strip_prefix(self.profile_folder())
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .into_owned();
                            self.commit(c);
                        })
                    }
                };
                if let Err(error) = result {
                    overlay::message(&error);
                }
                true
            }
            "app-pick" => {
                self.custom_app = value.parse().ok();
                self.push(Level::Page("custom-app"));
                false
            }
            "app-remove" => {
                let Some(i) = self
                    .custom_app
                    .filter(|&i| i < self.config.applications.len())
                else {
                    return true;
                };
                if !self.twice(page, value, "delete this entry") {
                    return false;
                }
                let mut c = self.config.clone();
                c.applications.remove(i);
                self.remember("custom-apps");
                if self.commit(c) {
                    self.refresh_apps();
                    self.custom_app = None;
                    if let Some(level) = overlay::back() {
                        let view = self.view(&level);
                        overlay::returned(view);
                    }
                    overlay::message("Deleted. The program and its files were not touched.");
                }
                false
            }
            "app-browse" => {
                overlay::hide();
                let picked = applications_ui::choose(null_mut());
                let editing = value == "change";
                let result = match picked {
                    Ok(None) => Ok(None),
                    Err(e) => Err(e),
                    Ok(Some(path)) => {
                        let text = path
                            .strip_prefix(self.profile_folder())
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .into_owned();
                        self.save_custom_app(if editing { "app-target" } else { "app-new" }, text)
                            .map(Some)
                    }
                };
                self.open_level(Level::Page("custom-apps"), true);
                if editing || matches!(result, Ok(Some(_))) {
                    self.push(Level::Page("custom-app"));
                }
                match result {
                    Ok(Some(text)) => overlay::message(&text),
                    Ok(None) => {}
                    Err(e) => overlay::message(&e),
                }
                false
            }
            "app-text" => {
                let text = if value == "clear" {
                    String::new()
                } else {
                    overlay::filter()
                };
                match self.save_custom_app(key, text) {
                    Ok(done) => {
                        if let Some(level) = overlay::back() {
                            let view = self.view(&level);
                            overlay::returned(view);
                        }
                        if key == "app-new" {
                            self.push(Level::Page("custom-app"));
                        }
                        overlay::message(&done);
                    }
                    Err(e) => overlay::message(&e),
                }
                false
            }
            "bar-pick" => {
                let mut parts = value.split(':').map(|v| v.parse::<usize>().unwrap_or(0));
                self.bar_pick = (parts.next().unwrap_or(0).min(2), parts.next().unwrap_or(0));
                self.push(Level::Page("bar-module"));
                false
            }
            "bar-add-open" => {
                self.bar_pick = (value.parse::<usize>().unwrap_or(0).min(2), 0);
                self.push(Level::Page("bar-add"));
                false
            }
            "bar-add" | "bar-move" => {
                let (s, i) = self.bar_pick;
                let mut c = self.config.clone();
                let mut leave = false;
                let result = if page == "bar-add" {
                    let m = bar_editor::modules()
                        .get(value.parse::<usize>().unwrap_or(usize::MAX))
                        .copied()
                        .ok_or_else(|| "Choose a module.".to_string());
                    leave = true;
                    m.and_then(|m| {
                        c.bar.add_module(s, m).map(|()| {
                            format!(
                                "Added {} to the {}.",
                                m.title(),
                                page_title(BAR_SECTIONS[s]).to_lowercase()
                            )
                        })
                    })
                } else {
                    match c.bar.section(s).get(i).copied() {
                        None => Err("That module is no longer in this section.".to_string()),
                        Some(m) => match value {
                            "up" | "down" => {
                                self.bar_pick.1 =
                                    c.bar
                                        .reorder_module(s, i, if value == "up" { -1 } else { 1 });
                                Ok("Moved.".to_string())
                            }
                            "remove" => {
                                c.bar.remove_module(s, i);
                                leave = true;
                                Ok(format!("Removed {} from the bar.", m.title()))
                            }
                            t => {
                                let t = t.parse::<usize>().unwrap_or(0).min(2);
                                c.bar.remove_module(s, i);
                                leave = true;
                                c.bar.add_module(t, m).map(|()| {
                                    format!(
                                        "Moved {} to the {}.",
                                        m.title(),
                                        page_title(BAR_SECTIONS[t]).to_lowercase()
                                    )
                                })
                            }
                        },
                    }
                };
                let result = result.and_then(|text| c.bar.validate().map(|()| text));
                match result {
                    Err(e) => {
                        overlay::message(&e);
                        true
                    }
                    Ok(text) => {
                        self.remember("bar-items");
                        if !self.commit(c) {
                            return true;
                        }
                        if leave {
                            if let Some(level) = overlay::back() {
                                let view = self.view(&level);
                                overlay::returned(view);
                            }
                        }
                        overlay::message(&text);
                        !leave
                    }
                }
            }
            "ws-desktop" => {
                self.layout_all_desktops = !self.layout_all_desktops;
                true
            }
            "ws-display" => {
                self.next_layout_monitor();
                true
            }
            "ws-layout" | "ws-remove" => {
                let mut candidate = self.layout_preferences.clone();
                let result = if page == "ws-remove" {
                    let index = value.parse::<usize>().unwrap_or(usize::MAX);
                    if index >= candidate.entries.len() {
                        return true;
                    }
                    if !self.twice(page, value, "remove this saved choice") {
                        return false;
                    }
                    candidate.entries.remove(index);
                    Ok("Removed. That scope now uses the inherited layout.")
                } else {
                    match self.layout_page_scope() {
                        None => Err("Windows desktop information is unavailable.".to_string()),
                        Some((desktop, monitor)) => {
                            let layout = (!value.is_empty()).then(|| value.to_string());
                            candidate.set(desktop, monitor, layout).map(|()| {
                                "Saved on this PC. The selected scope now uses the chosen or inherited layout."
                            })
                        }
                    }
                };
                let result =
                    result.and_then(|text| self.save_layout_choices(candidate).map(|()| text));
                overlay::message(&match result {
                    Ok(text) => text.to_string(),
                    Err(error) => error,
                });
                true
            }
            "terminal-colors" => {
                let result = match value {
                    "apply" => {
                        appearance_effects::apply_terminal(&self.directory, &self.config.theme).map(
                            |()| "Added. Restart Windows Terminal to see the Omarchy Overlay profile.",
                        )
                    }
                    "restore" => appearance_effects::restore_terminal(&self.directory)
                        .map(|()| "Removed. Restart Windows Terminal."),
                    _ => {
                        if shell_in(Path::new("wt.exe"), "-p \"Omarchy Overlay\"", None) {
                            overlay::hide();
                            return false;
                        }
                        Err("Windows Terminal could not open. Add the themed profile first, then restart Terminal if needed.".into())
                    }
                };
                overlay::message(&match result {
                    Ok(text) => text.to_string(),
                    Err(error) => error,
                });
                true
            }
            "windows-theme" => {
                let result = if value == "mode" {
                    appearance_effects::with_effects(|| windows_personalization::apply(&self.directory, theme::current().dark))
                } else {
                    let on = windows_personalization::current(value) != Some(1);
                    appearance_effects::with_effects(|| windows_personalization::apply_effect(&self.directory, value, on))
                };
                overlay::message(&match result {
                    Ok(()) => "Applied. Restore puts back the original.".into(),
                    Err(error) => error,
                });
                true
            }
            "webapps-remove" => {
                if !self.twice(page, value, "remove it from Omarchy Overlay") {
                    return false;
                }
                self.remember(key);
                self.webapps.retain(|w| w.name != value);
                self.save_webapps();
                true
            }
            "install-webapp" => {
                let Some(app) = self.webapps.iter().find(|w| w.name == value).cloned() else {
                    return false;
                };
                if !shell(Path::new("chrome.exe"), &format!("\"{}\"", app.url))
                    && !shell(Path::new(&app.url), "")
                {
                    overlay::message("Chrome could not be opened.");
                    return false;
                }
                overlay::message(&format!(
                    "In Chrome: ⋮ → Cast, save and share → Install page as app, named \"{}\".",
                    app.shortcut()
                ));
                false
            }
            "remove-apps" => {
                self.remember(key);
                let hidden = self.picker_history.hidden(value);
                self.picker_history.set_hidden(value, !hidden);
                self.save_picker();
                self.filter_apps();
                true
            }
            "add-rule" => {
                let h = value.parse::<isize>().unwrap_or(0) as HWND;
                if let Some(i) = self.rule_index(h) {
                    self.push(Level::Rule(Some(i)));
                    return false;
                }
                let exe = process_name(h);
                if !exe.to_ascii_lowercase().ends_with(".exe") {
                    overlay::message("Omarchy Overlay cannot read this window's program name. It may run as administrator.");
                    return false;
                }
                self.remember("app-rules");
                let mode = window_policy(&self.config, h).mode;
                self.save_app_rule(
                    None,
                    AppRule {
                        executable: exe,
                        mode,
                        ..Default::default()
                    },
                );
                self.push(Level::Rule(Some(self.config.rules.len() - 1)));
                false
            }
            "rule" | "rule-mode" | "rule-active" | "rule-inactive" | "rule-material"
            | "rule-theme" | "rule-opacity" => self.change_rule(page, value),
            "panel" | "panel-mode" | "panel-opacity" => self.panel_action(page, value),
            "profiles" => {
                if value == "files" {
                    overlay::hide();
                    self.show_profiles();
                    return false;
                }
                self.remember(key);
                let mut c = self.config.clone();
                let name = (1..)
                    .map(|n| format!("My profile {n}"))
                    .find(|n| {
                        !self
                            .profile_list()
                            .iter()
                            .any(|p| p.name.eq_ignore_ascii_case(n))
                    })
                    .unwrap_or_default();
                c.appearance_profiles
                    .push(Profile::capture(&c, name.clone()));
                if self.commit(c) {
                    self.last_profile = Some(name.clone());
                    overlay::message(&format!(
                        "Saved as \"{name}\". Rename it under Profile files."
                    ));
                }
                true
            }
            "profile" => self.profile_action(value),
            "restore-failed" => {
                let keys: Vec<isize> = if value == "all" {
                    self.saved.keys().copied().collect()
                } else {
                    vec![value.parse().unwrap_or(0)]
                };
                for k in keys {
                    self.restore_one(k);
                }
                overlay::message(if self.saved.is_empty() {
                    "Every window is back in place."
                } else {
                    "Some windows still could not be restored. Move them by hand."
                });
                true
            }
            "version" => {
                overlay::hide();
                crate::desktops::follow(self.main);
                // K6: opens on the primary monitor like the other settings windows.
                panel_scroll::fit(self.main);
                ShowWindow(self.main, SW_SHOWNORMAL);
                SetForegroundWindow(self.main);
                false
            }
            _ => false,
        }
    }
    unsafe fn change_rule(&mut self, page: &'static str, value: &str) -> bool {
        let Some(Level::Rule(Some(i))) = overlay::level() else {
            return false;
        };
        let Some(mut rule) = self.config.rules.get(i).cloned() else {
            return false;
        };
        match (page, value) {
            ("rule-mode", m) => rule.mode = mode_of(m),
            ("rule-active", _) => {
                rule.active_opacity = cycle(
                    &rule.active_opacity,
                    &[None, Some(100), Some(95), Some(90), Some(85)],
                )
            }
            ("rule-inactive", _) => {
                rule.inactive_opacity = cycle(
                    &rule.inactive_opacity,
                    &[
                        None,
                        Some(100),
                        Some(95),
                        Some(90),
                        Some(85),
                        Some(80),
                        Some(70),
                    ],
                )
            }
            ("rule-material", _) => {
                rule.backdrop = cycle(
                    &rule.backdrop,
                    &[
                        None,
                        Some("native".into()),
                        Some("mica".into()),
                        Some("acrylic".into()),
                    ],
                )
            }
            ("rule-theme", _) => {
                rule.coordinate_theme =
                    cycle(&rule.coordinate_theme, &[None, Some(true), Some(false)])
            }
            ("rule-opacity", _) => rule.allow_opacity = !rule.allow_opacity,
            ("rule", "pair") => {
                let other = if rule.executable.eq_ignore_ascii_case("olk.exe") {
                    "OUTLOOK.EXE"
                } else {
                    "olk.exe"
                };
                self.remember("rule");
                let n = self.save_app_rule(
                    None,
                    AppRule {
                        executable: other.into(),
                        ..rule
                    },
                );
                overlay::message(&format!("Added. {n} open {other} window(s) follow it now."));
                return true;
            }
            ("rule", "delete") => {
                if !self.twice(page, value, "delete this rule") {
                    return false;
                }
                self.remember("rule");
                let mut c = self.config.clone();
                c.rules.remove(i);
                if self.commit(c) {
                    if let Some(level) = overlay::back() {
                        let view = self.view(&level);
                        overlay::returned(view);
                    }
                    overlay::message("Rule deleted.");
                }
                return false;
            }
            _ => return false,
        }
        self.remember("rule");
        let n = self.save_app_rule(Some(i), rule);
        overlay::message(&format!("Saved. {n} open window(s) follow this rule now."));
        true
    }
    unsafe fn panel_action(&mut self, page: &'static str, value: &str) -> bool {
        let Some(Level::WindowPanel(h)) = overlay::level() else {
            return false;
        };
        let h = h as HWND;
        if IsWindow(h) == 0 {
            overlay::message("That window has closed.");
            return false;
        }
        match (page, value) {
            ("panel-mode", m) => {
                let exe = process_name(h);
                if !exe.to_ascii_lowercase().ends_with(".exe") {
                    overlay::message("Omarchy Overlay cannot read this window's program name. It may run as administrator.");
                    return false;
                }
                let index = self.config.rules.iter().position(|r| {
                    r.executable.eq_ignore_ascii_case(&exe)
                        && r.title_contains.is_empty()
                        && r.window_class.is_empty()
                });
                let mut rule = index
                    .map(|i| self.config.rules[i].clone())
                    .unwrap_or(AppRule {
                        executable: exe,
                        ..Default::default()
                    });
                rule.mode = mode_of(m);
                self.remember("panel");
                let n = self.save_app_rule(index, rule);
                overlay::message(&format!(
                    "Saved. {n} open window(s) of this app follow it now."
                ));
                true
            }
            ("panel-opacity", p) => {
                self.opacity_preset(h, p.parse().unwrap_or(100));
                true
            }
            ("panel", "float") => {
                self.float_window(h);
                true
            }
            ("panel", "rule") => {
                let i = match self.rule_index(h) {
                    Some(i) => i,
                    None => {
                        let exe = process_name(h);
                        if !exe.to_ascii_lowercase().ends_with(".exe") {
                            overlay::message("Omarchy Overlay cannot read this window's program name. It may run as administrator.");
                            return false;
                        }
                        let mode = window_policy(&self.config, h).mode;
                        self.save_app_rule(
                            None,
                            AppRule {
                                executable: exe,
                                mode,
                                ..Default::default()
                            },
                        );
                        self.config.rules.len() - 1
                    }
                };
                self.push(Level::Rule(Some(i)));
                false
            }
            ("panel", "always-float") => {
                let mut c = self.config.clone();
                c.always_float.windows.push(crate::always_float::Entry {
                    executable: process_name(h),
                    window_class: window_class(h),
                    title_contains: String::new(),
                });
                self.remember("panel");
                if self.commit(c) {
                    overlay::message("Added to the always-float list.");
                }
                true
            }
            ("panel", action) => {
                overlay::hide();
                SetForegroundWindow(h);
                match action {
                    "maximize" => self.maximize_window(h),
                    "main" => self.promote(h),
                    "monitor" => self.move_monitor(h, 1),
                    "scratchpad" => self.assign_scratchpad(h),
                    _ => self.close_window(h),
                }
                false
            }
            _ => false,
        }
    }
    unsafe fn profile_action(&mut self, value: &str) -> bool {
        let Some(Level::Profile(i)) = overlay::level() else {
            return false;
        };
        let Some(profile) = self.profile_list().get(i).cloned() else {
            return false;
        };
        let custom = i.checked_sub(appearance_profiles::THEMES.len());
        match (value, custom) {
            ("apply", _) => {
                self.remember("profile");
                match self.activate_appearance(&profile) {
                    Ok(()) => {
                        self.last_profile = Some(profile.name.clone());
                        overlay::set_font(&self.config.menu_font, self.config.menu_font_size);
                        overlay::message(&format!("Applied \"{}\".", profile.name));
                    }
                    Err(error) => overlay::message(&error),
                }
                true
            }
            ("update", Some(j)) => {
                self.remember("profile");
                let mut c = self.config.clone();
                c.appearance_profiles[j] = Profile::capture(&c, profile.name.clone());
                if self.commit(c) {
                    overlay::message(&format!(
                        "\"{}\" now holds the current settings.",
                        profile.name
                    ));
                }
                true
            }
            ("delete", Some(j)) => {
                if !self.twice("profile", value, "delete this profile") {
                    return false;
                }
                self.remember("profile");
                let mut c = self.config.clone();
                c.appearance_profiles.remove(j);
                if self.commit(c) {
                    if self.last_profile.as_deref() == Some(profile.name.as_str()) {
                        self.last_profile = None;
                    }
                    if let Some(level) = overlay::back() {
                        let view = self.view(&level);
                        overlay::returned(view);
                    }
                    overlay::message("Profile deleted.");
                }
                false
            }
            _ => false,
        }
    }
    /// Opens one of the user's config files in Notepad, starting it from the example if there is none.
    pub(super) unsafe fn edit_config_file(&self, name: &str) {
        let path = config_folder::folder().join(name);
        if !path.exists() {
            let example = config_folder::folder().join("examples").join(name);
            let text = fs::read_to_string(&example).unwrap_or_default();
            if let Err(error) = config_folder::save(name, &text) {
                overlay::message(&error);
                return;
            }
        }
        overlay::hide();
        if !shell(Path::new("notepad.exe"), &format!("\"{}\"", path.display())) {
            message(&format!("Notepad could not open {}.", path.display()));
        }
    }
    /// Starts an app picker entry, the same way the 0.15 picker did.
    pub(super) unsafe fn launch_entry(&mut self, index: usize) {
        let Some(entry) = self.entries.get(index).cloned() else {
            return;
        };
        if let Some(action) = entry.action.clone() {
            self.picker_history.launched(&entry.key());
            self.save_picker();
            self.run_menu_action(action);
            return;
        }
        if !entry.available() {
            message(&format!(
                "This custom entry is unavailable on this PC. Check its path and working folder in Custom apps.\r\n{}",
                entry.path.display()
            ));
            return;
        }
        if shell_in(
            &entry.path,
            &entry.arguments,
            entry.working_directory.as_deref(),
        ) {
            self.picker_history.launched(&entry.key());
            self.save_picker();
        } else {
            message(&format!(
                "Could not launch {}. Its shortcut may no longer be valid.",
                entry.label
            ));
        }
    }
}

fn windows_view() -> View {
    let tasks = unsafe { tasks::collect() };
    let mut rows: Vec<Row> = tasks
        .iter()
        .map(|t| {
            task_row(t, t.title.clone(), Act::Window(t.hwnd as isize))
                .detail(t.identity.label.clone())
        })
        .collect();
    if rows.is_empty() {
        rows.push(info("No open windows."));
    }
    View {
        title: "Open windows".into(),
        rows,
        hint: "Enter switches · type to search · Esc goes back".into(),
        ranked: false,
    }
}
