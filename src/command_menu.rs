//! Searchable actions share the existing picker, not a shell-command interpreter.
use super::*;
#[derive(Clone)]
pub(super) enum Action {
    Command(u32),
    Window(tasks::Task),
}
impl Action {
    pub(super) fn key(&self) -> String {
        match self {
            Self::Command(id) => format!("command:{id}"),
            Self::Window(task) => format!("window:{}:{}", task.hwnd as isize, task.pid),
        }
    }
}
pub(super) unsafe fn entries() -> Vec<Entry> {
    let commands = &[
        (0, "Toggle tiling"),
        (1, "Settings and app rules"),
        (2, "Appearance"),
        (3, "Bar and workspaces"),
        (4, "Keyboard shortcuts"),
        (5, "Layouts by workspace"),
        (6, "Manage custom apps"),
        (7, "Restore desktop and exit"),
        (8, "Windows Task View"),
        (9, "Rebalance tiles"),
        (10, "Layout main and stack"),
        (11, "Layout columns"),
        (12, "Layout rows"),
        (13, "Layout grid"),
        (14, "Layout dwindle"),
        (15, "Layout scrolling — Niri-inspired"),
        (20, "Theme Everforest"),
        (21, "Theme Tokyo Night"),
        (22, "Theme Nord"),
        (23, "Theme Flexoki Light"),
        (24, "Theme Alpenglow Night"),
        (25, "Appearance profiles"),
        (26, "Theme Neon Ridge"),
        (30, "Move selected window to left monitor"),
        (31, "Move selected window to right monitor"),
        (32, "Move selected window to upper monitor"),
        (33, "Move selected window to lower monitor"),
        (34, "Close selected window"),
        (35, "Maximize or restore selected window"),
        (36, "Assign selected window as scratchpad"),
        (37, "Toggle scratchpad"),
        (38, "Windows clipboard history"),
        (39, "Windows screen capture"),
        (40, "Flip selected dwindle split"),
        (41, "Resize selected window narrower"),
        (42, "Resize selected window wider"),
        (43, "Resize selected window shorter"),
        (44, "Resize selected window taller"),
        (45, "Toggle selected window 85% opacity"),
        (46, "Toggle selected window 90% opacity"),
        (47, "Toggle selected window 95% opacity"),
        (48, "Toggle selected window 98% opacity"),
        (60, "Windows taskbar visible"),
        (61, "Windows taskbar auto-hide"),
        (62, "Windows taskbar keep hidden"),
        (70, "Apps and global defaults"),
        (71, "Edit bar modules"),
        (72, "Windows notifications"),
        (73, "Windows Quick Settings"),
        (74, "Windows display settings"),
        (75, "Windows power-user menu"),
        (76, "Windows settings"),
        (77, "Materials, Windows theme and motion"),
    ];
    let mut entries = commands
        .iter()
        .map(|(id, label)| Entry {
            label: format!("Action: {label}"),
            path: PathBuf::new(),
            arguments: String::new(),
            custom_key: None,
            working_directory: None,
            action: Some(Action::Command(*id)),
        })
        .collect::<Vec<_>>();
    entries.extend(tasks::collect().into_iter().map(|task| Entry {
        label: format!("Window: {}", task.title),
        path: PathBuf::new(),
        arguments: String::new(),
        custom_key: None,
        working_directory: None,
        action: Some(Action::Window(task)),
    }));
    entries
}
impl App {
    pub(super) unsafe fn run_menu_action(&mut self, action: Action) {
        match action {
            Action::Window(task) => {
                if !tasks::activate(&task, false) {
                    message("That window is no longer available. Reopen the menu to refresh it.");
                }
            }
            Action::Command(id) => {
                let target = self
                    .menu_target
                    .as_ref()
                    .filter(|task| task.valid())
                    .map_or(null_mut(), |task| task.hwnd);
                match id {
                    0 => self.toggle(),
                    // Settings results open the matching Omarchy Overlay menu list.
                    1 => self.open_menu("setup"),
                    2 => self.open_menu("style"),
                    3 => self.open_menu("bar"),
                    4 => self.open_level(overlay::Level::Keybinds, true),
                    5 => self.show_workspace_layouts(),
                    6 => self.show_applications(),
                    7 => PostQuitMessage(0),
                    8 => crate::desktops::task_view(),
                    9 => self.rebalance(),
                    10..=15 => self.set_layout(
                        [
                            "main-stack",
                            "columns",
                            "rows",
                            "grid",
                            "dwindle",
                            "scrolling",
                        ][(id - 10) as usize],
                    ),
                    25 => self.open_level(overlay::Level::Page("profiles"), true),
                    26 => self.apply_theme("neon-ridge"),
                    20..=24 => self.apply_theme(
                        crate::appearance_profiles::COMMAND_THEMES[(id - 20) as usize],
                    ),
                    30 => self.move_direction(target, -1, 0),
                    31 => self.move_direction(target, 1, 0),
                    32 => self.move_direction(target, 0, -1),
                    33 => self.move_direction(target, 0, 1),
                    34 => self.close_window(target),
                    35 => self.maximize_window(target),
                    36 => self.assign_scratchpad(target),
                    37 => self.toggle_scratchpad(),
                    38 => self.queue_system_action(0),
                    39 => self.queue_system_action(1),
                    40 => self.change_split(target, true, 0.),
                    41 => self.resize_keyboard(target, true, -40),
                    42 => self.resize_keyboard(target, true, 40),
                    43 => self.resize_keyboard(target, false, -40),
                    44 => self.resize_keyboard(target, false, 40),
                    60 => self.set_taskbar_policy("visible"),
                    61 => self.set_taskbar_policy("auto-hide"),
                    62 => self.set_taskbar_policy("hidden"),
                    70 => self.show_app_rules(),
                    71 => self.show_bar_editor(),
                    72 => self.queue_system_action(4),
                    73 => self.queue_system_action(3),
                    74 => {
                        shell(Path::new("ms-settings:display"), "");
                    }
                    75 => self.queue_system_action(2),
                    77 => self.show_appearance_options(),
                    76 => {
                        shell(Path::new("ms-settings:"), "");
                    }
                    45..=48 => self.opacity_preset(target, [85, 90, 95, 98][(id - 45) as usize]),
                    _ => {}
                }
            }
        }
    }
}
