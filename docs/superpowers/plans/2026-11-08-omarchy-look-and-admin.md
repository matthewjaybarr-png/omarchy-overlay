# Omarchy look, themes and admin mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Omarchy themes built in, an Omarchy/Hyprland look on Windows 11, and two releases from one exe: a portable Standard edition that never needs admin, and a self-installing elevated Admin edition, both without the work-build restrictions.

**Architecture:** A Python script converts the laptop's Omarchy themes into embedded `colors.toml` files plus a generated Rust table; the existing theme, frame (`window_theme.rs`), bar and personalization code is extended rather than replaced. The installer lives in the exe (`--install`/`--uninstall`) and uses a highest-privileges logon task; launches from an elevated instance go through an unelevated helper copy of the exe parented to Explorer. The edition is not a build flag: elevated behavior follows `process_elevated()`, installed behavior follows the exe's folder, and an `edition-admin` marker file in the Admin zip only adds the tray's install item, so one build and one test run cover both.

**Tech Stack:** Rust 2021 + windows-sys (cross-built with `cargo xwin`), Python 3 scripts, PowerShell VM tests over libvirt (`scripts/vm.py`, `scripts/test_windows.py`).

**Spec:** `docs/superpowers/specs/2026-11-08-omarchy-look-and-admin-design.md`

## Global Constraints

- Repo is public: no employer names, branding or work details in code, docs, commit messages or embedded files.
- Never touch the private build; never push from a session.
- Theme skip list: `chatgpt-image-*`, `wallhaven-*`, `matt1`. Neon Ridge wallpapers `01-*`, `02-*`, `03-*` never copied. Wallpapers never enter the repo.
- Every change to another app's window or Windows setting is journaled first and restored by exit, `--recover`, `RECOVER.cmd` and uninstall.
- Ctrl+Alt+Delete and Super+L always stay with Windows.
- Gate per stage: `cargo test --offline`, `cargo clippy --offline --target x86_64-pc-windows-msvc -- -D warnings`, release build `cargo +1.99.1 xwin build --release --target x86_64-pc-windows-msvc --offline`, VM suites with `OVERLAY_VM="Windows 11" OVERLAY_VM_USER=Matt OVERLAY_VM_GUEST='C:\Users\Matt\Overlay-Test'`, one commit per task, stage ends with a CHANGELOG entry.
- VM users: Standard edition as `worktest` (standard user, signed in at the VM console first; `OVERLAY_VM_USER=worktest OVERLAY_VM_GUEST='C:\Users\worktest\Overlay-Test'`, see `~/Windows-VM-Testing/README.md`); Admin edition as `Matt` (or `tbtest`).
- Identity stays separate from the private build: own exe name, window classes, mutex, `%LOCALAPPDATA%\OmarchyOverlay`, task name, install dir.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A saved profile whose theme was a removed built-in (`jade`, `paper`, `tokyo-night` id unchanged) — must load and fall back to Neon Ridge, not fail validation. (Task 2 test)
2. A user theme folder with the same id as a built-in — the user copy wins, no "rename the file" error. (Task 2 test)
3. App crashes while borders are applied — `--recover` resets border color and corners on every recorded window. (Task 4 recovery suite assertion)
4. Uninstall while the app is running from `Program Files` — app stops, desktop restored, folder removed after exit. (Task 11 VM test)
5. Elevated instance launching a terminal or URL — child is not elevated, arguments with spaces and quotes survive. (Task 10 test)
6. Standard zip run by a standard user — never prompts for UAC, never writes to `Program Files`, `HKLM` or `%ProgramData%`, and shows no install item. (Task 13 VM test as `worktest`)

---

## Stage 1 — Themes

### Task 1: Theme import script and embedded theme table

**Files:**
- Create: `scripts/import_omarchy_themes.py`, `themes/omarchy/<id>.toml` (generated), `src/omarchy_themes.rs` (generated)
- Modify: `src/main.rs` (add `mod omarchy_themes;` beside the other modules)

**Interfaces:**
- Produces: `pub(crate) const NAMES: &[(&str, &str)]` (id, label) and `pub(crate) const FILES: &[(&str, &str)]` (id, colors.toml text) in `src/omarchy_themes.rs`, sorted by id, `neon-ridge` first in `NAMES`.
- CLI: `python3 scripts/import_omarchy_themes.py [--out DIR]`; sources default to `/usr/share/omarchy/themes` and `~/.config/omarchy/themes` (later source wins on id clash).

- [ ] **Step 1:** Write the script with a `selftest()` run by `--selftest`: asserts `skipped('chatgpt-image-aug-23-x')`, `skipped('wallhaven-d4v2pn')`, `skipped('matt1')`, `not skipped('neon-ridge')`; `banned_wallpaper('neon-ridge', '02-peak.jpg')` true, `banned_wallpaper('neon-ridge', '10-wallpaper.png')` false; `strip_comments('# a\nx = "#000000"\n') == 'x = "#000000"\n'`.
- [ ] **Step 2:** Run `python3 scripts/import_omarchy_themes.py --selftest` → expect failure until implemented, then `selftest ok`.
- [ ] **Step 3:** Implement: for each allowed theme dir with `colors.toml`, write comment-stripped text to `themes/omarchy/<id>.toml`; label = the theme's dir name title-cased by dash (same rule as `theme_files::parse`); generate `src/omarchy_themes.rs` with `include_str!("../themes/omarchy/<id>.toml")`; skip and print themes lacking `colors.toml` or ids failing `[a-z0-9-]{1,64}`. With `--out DIR` also write `DIR/<id>/colors.toml` and copy `backgrounds/*` minus banned files (follow symlinks).
- [ ] **Step 4:** Run the script; expect 30 themes (24 stock + aether, alpenglow-night, haunted-town, machine-shop, neon-ridge, purplemountain; fewer if one lacks colors.toml — the script prints why).
- [ ] **Step 5:** Commit script, `themes/omarchy/`, `src/omarchy_themes.rs`.

### Task 2: Runtime themes from embedded files and Omarchy folders

**Files:**
- Modify: `src/theme_files.rs`, `src/theme.rs`, `src/config_folder.rs:74-100`, `assets/neon-ridge-colors.toml` (delete; tests use `omarchy_themes::FILES`), `config/examples/themes/*` + `config/examples/README.txt` (describe folder layout)

**Interfaces:**
- Consumes: `omarchy_themes::{NAMES, FILES}`.
- Produces: `theme_files::Theme` gains `pub colors: std::collections::BTreeMap<String, u32>` (every color key) and `pub backgrounds: Option<std::path::PathBuf>`; `theme::NAMES` becomes `pub(super) use crate::omarchy_themes::NAMES;` (callers in `windows.rs`, `overlay_pages.rs`, `profiles_actions.rs` unchanged); `theme::backgrounds(name: &str) -> Option<PathBuf>`.

- [ ] **Step 1: Tests in `theme_files.rs`**
  - `every_built_in_theme_parses`: every `FILES` entry parses; `neon-ridge` accent `0x38bdfa`, surface `0x1a2550`, `colors["magenta"] == 0xb580fa`.
  - `unknown_keys_are_ignored`: `parse("x", "background=\"#000000\"\nforeground=\"#ffffff\"\naccent=\"#ff0000\"\nnew_thing = 3\n")` is Ok (non-color values on unknown keys ignored; a bad value on a known color key still errors with its line).
  - `user_theme_overrides_built_in` (in `theme.rs` tests): `install(vec![parse("tokyo-night", <accent #123456>)])`, `palette("tokyo-night").accent == rgb(0x123456)`.
  - `removed_ids_fall_back`: `set("paper") == false` and `current()` equals `palette("neon-ridge")`.
- [ ] **Step 2:** `cargo test --offline theme` → fail.
- [ ] **Step 3:** Implement. `palette(name)`: user-loaded theme first, then parse of the embedded `FILES` entry (cache in a `OnceLock<Vec<Theme>>`); delete the hard-coded match. `config_folder::themes` also reads `themes\<id>\colors.toml` directories (setting `backgrounds` when `themes\<id>\backgrounds` exists), drops the "is a built-in theme" error, keeps the 64 cap. `windows_accent` keeps its Neon Ridge special case.
- [ ] **Step 4:** `cargo test --offline` and clippy pass.
- [ ] **Step 5:** Commit.

### Task 3: Stage 1 VM evidence

- [ ] **Step 1:** Release build; `python3 scripts/import_omarchy_themes.py --out <scratch>/themes`; push that folder to the guest config `themes\` (`scripts/vm.py push`).
- [ ] **Step 2:** Make the `workspaces` rollback check optional in `scripts/test_windows.py:57-59` (skip the push and pass `-SkipRollback` when `--rollback-exe` is missing; add the matching switch to `tests/windows-workspaces.ps1`).
- [ ] **Step 3:** Run `python3 scripts/test_windows.py smoke customization appearance-integration`; expect `All requested native suites passed.` Screenshot the menu under Tokyo Night, Catppuccin Latte and Neon Ridge (Neon Ridge background from the pushed folder).
- [ ] **Step 4:** README "themes" section + CHANGELOG 0.2.0 stage-1 entry; commit.

## Stage 2 — Look

### Task 4: Hyprland borders, corners and gaps

**Files:**
- Modify: `src/window_theme.rs` (`ATTRS`, desired values, `poll` signature), `src/windows.rs:1928` (caller), `src/config.rs` (fields, defaults, validation), `tests/windows-recovery.ps1`

**Interfaces:**
- `Config` gains `pub window_borders: bool` (default true) and `pub outer_gap: i32` (default 10, valid 0..=80); `gap` default becomes 5. Both `#[serde(default)]` so old profiles load.
- `Engine::poll(&mut self, windows: &[HWND], focused: HWND, policy: impl Fn(HWND) -> Option<(bool, u32, bool)>)` — third tuple field = borders on.

- [ ] **Step 1: Tests:** `config.rs`: default `gap == 5`, `outer_gap == 10`, `window_borders`; an old profile JSON without the fields deserializes with those defaults; `outer_gap = 81` fails validation. Layout test (in `layout.rs` tests): outer edge offset uses `outer_gap`, between-window spacing uses `gap`.
- [ ] **Step 2:** Run → fail.
- [ ] **Step 3:** Implement. `ATTRS = [20, 33, 34, 35, 36, 38]`; when borders on: `33 => 1` (DWMWCP_DONOTROUND), `34 => accent` if focused else `theme::current().border`. Recovery validation already checks against `ATTRS`. Layout uses `outer_gap` at monitor edges.
- [ ] **Step 4:** Extend `tests/windows-recovery.ps1`: kill the app with borders on, run `--recover`, assert `DwmGetWindowAttribute(34)` returns `0xFFFFFFFF` (DWMWA_COLOR_DEFAULT) and `33` returns 0 on the test window.
- [ ] **Step 5:** `cargo test`, clippy; commit.

### Task 5: Waybar-style bar defaults

**Files:** Modify `src/bar_model.rs` (`Default for BarConfig`), `src/config.rs` (bar opacity default), bar paint in `src/bar.rs:1233-1380` only if a value below is not configurable.

- [ ] **Step 1: Test** in `bar_model.rs`: default `left == [Workspaces]`, `center == [Clock]`, `right == [SystemTray, Sound, Network, Power]`, `font_family == "JetBrainsMono Nerd Font"`, `font_size == 14`, clock format `"time-24h"` if listed else `"day-time"`; config default `bar_opacity == 79`.
- [ ] **Step 2:** Run → fail. **Step 3:** Change defaults; active workspace drawn in accent, others in `muted`. **Step 4:** tests pass. **Step 5:** Commit.

### Task 6: Walker-style launcher and menus

**Files:** Modify `src/config.rs` (menu defaults), `src/overlay.rs:440-470` (frame), `src/picker_ui.rs`.

- [ ] **Step 1: Test** in `config.rs`: default `menu_font == "JetBrainsMono Nerd Font"`, `menu_font_size == 14`.
- [ ] **Step 2–3:** Overlay/picker: centered on the active monitor, square corners (33 = 1), 2 px accent border (34 = accent), background alpha 84% (attribute/`SetLayeredWindowAttributes`), selected row filled with `selection`/`border` and text in `accent`.
- [ ] **Step 4:** tests + clippy; commit.

### Task 7: Theme switch drives Windows

**Files:** Modify `src/windows.rs:1856-1900` (`apply_theme`), `src/appearance_effects.rs` (wallpaper setter reuse), `src/commands.rs` (new command).

**Interfaces:** command id `next-background` ("Next theme background"), default bind `Super+Ctrl+Space` in `config/examples/binds.conf`.

- [ ] **Step 1: Test** a pure helper `fn next_background(list: &[PathBuf], current: Option<&Path>) -> Option<PathBuf>`: empty → None; current absent → first; last → wraps to first.
- [ ] **Step 2:** Run → fail.
- [ ] **Step 3:** `apply_theme`: when `theme::backgrounds(name)` has images, set the first one through the existing journaled wallpaper path; set accent via `windows_accent` and dark/light via `windows_personalization::apply` always (journal created on first use), not only when the restore file already exists. `next-background` cycles with the helper.
- [ ] **Step 4:** tests + clippy; commit.

### Task 8: Stage 2 VM evidence

- [ ] Release build; run `smoke recovery picker taskbar appearance-integration customization`; screenshots of bar + menu + focused/unfocused borders under two themes; confirm wallpaper/accent restore via the recovery menu. README "Look" section, CHANGELOG; commit.

## Stage 3 — Two editions and admin mode

### Task 9: Remove work-build restrictions

**Files:** `src/config.rs:83,231-260` (delete `always_ignored`), `src/app_rules_ui.rs:387,452`, `src/profiles_actions.rs:33`, `src/overlay_pages.rs:1080`, `src/binds.rs:2-3,110-122`, `src/profile.rs:185-215`, `src/windows.rs:713-718`, README.

**Interfaces:** `profile::find(executable, local, explicit) -> Found` (the `stick` parameter is removed; `ON_STICK` stays only if other code reads it). `Key::warning(self) -> Option<&'static str>` returns a warning for plain-Alt keys; `config_folder::load` logs it per bind.

- [ ] **Step 1: Tests:** an app rule set to Tile survives save/load; `Key::parse("Alt+J")` is Ok with a warning; `Key::parse("Ctrl+J")` still errors; Super+L and Ctrl+Alt+Delete still error; `profile::find` for `C:\Program Files\OmarchyOverlay\OmarchyOverlay.exe` with no settings folder returns `Found::Use` (local), and an exe with `OVERLAY SETTINGS` beside it on a fixed drive uses that folder.
- [ ] **Step 2:** Run → fail. **Step 3:** Implement. **Step 4:** tests + clippy. **Step 5:** Commit.

### Task 10: Unelevated launches from an elevated instance

**Files:** Create `src/launch_unelevated.rs`; modify `src/windows.rs:528-548` (`shell_in`), `src/windows.rs:604` (argument handling, before the mutex).

**Interfaces:**
- `pub(super) fn process_elevated() -> bool` (TokenElevation of the current process).
- `pub(super) unsafe fn shell_unelevated(path: &Path, arguments: &str, directory: Option<&Path>) -> bool`: starts `OmarchyOverlay.exe --shell-open <path> <arguments> <directory-or-empty>` via `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_PARENT_PROCESS` = Explorer's shell process (`GetShellWindow` → pid), so it inherits Explorer's token. The helper does the existing `ShellExecuteW` and exits without taking the instance mutex.
- `shell_in` calls `shell_unelevated` when `process_elevated()`; if it fails, logs and does **not** fall back to an elevated launch.
- Pure helpers with tests: `fn helper_command_line(exe: &Path, path: &Path, arguments: &str, directory: Option<&Path>) -> String` and `fn parse_helper_args(args: &[String]) -> Option<(PathBuf, String, Option<PathBuf>)>`; round-trip test with spaces, quotes and an empty directory.

- [ ] Steps: test → fail → implement → pass → commit. VM check in Task 12.

### Task 11: Self-installer

**Files:** Create `src/install.rs`; modify `src/windows.rs:604` (`--install`, `--uninstall`, `--uninstall --remove-settings`), tray menu (add "Install (admin)…" when not installed, "Uninstall…" when `current_exe` is under the install dir), README.

**Interfaces / fixed values:**
- Tray: "Install (admin)…" shows only when `edition-admin` exists beside the exe and the exe is not in the install dir; "Uninstall…" shows when it is. `fn tray_install_items(marker: bool, installed: bool) -> (bool, bool)` is pure and tested for all four cases. `--install`/`--uninstall` work regardless of the marker.
- Install dir: `%ProgramFiles%\OmarchyOverlay` (no space, so `profile::release_root` never matches). Copies `OmarchyOverlay.exe`, `fonts\`, `RECOVER.cmd`.
- Not elevated → relaunch self with `ShellExecuteW("runas", exe, "--install")` and exit.
- Logon task: `schtasks /Create /F /TN "Omarchy Overlay" /SC ONLOGON /RL HIGHEST /RU "<DOMAIN\user>" /IT /TR "\"<dir>\OmarchyOverlay.exe\""`; the installing user comes from the unelevated parent (pass `--user <name>` on the runas relaunch).
- Start menu: `%ProgramData%\Microsoft\Windows\Start Menu\Programs\Omarchy Overlay.lnk` — target `schtasks.exe /Run /TN "Omarchy Overlay"` (so it starts elevated without a prompt), icon from the exe; created with `IShellLinkW` or a one-line PowerShell `WScript.Shell` call.
- Uninstall key `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\OmarchyOverlay`: `DisplayName "Omarchy Overlay"`, `DisplayVersion` = crate version, `InstallLocation`, `DisplayIcon`, `UninstallString "\"<dir>\OmarchyOverlay.exe\" --uninstall"`, `NoModify=1`, `NoRepair=1`.
- `--uninstall`: post `WM_CLOSE` to the running main window (`FindWindowW(MAIN_CLASS)`) and wait ≤10 s, run the same restore as `--recover` plus `windows_personalization::restore`, delete task, shortcut, key; remove the dir via a detached `cmd /c ping -n 3 127.0.0.1>nul & rmdir /s /q "<dir>"`. `%LOCALAPPDATA%\OmarchyOverlay` kept unless `--remove-settings`.
- Pure, tested: `fn schtasks_create_args(exe: &Path, user: &str) -> Vec<String>`, `fn uninstall_values(dir: &Path, version: &str) -> Vec<(&'static str, String)>`.

- [ ] Steps: tests → fail → implement → pass → clippy → commit.

### Task 12: Two release packages

**Files:** Create `scripts/package.py`, `packaging/INSTALL.cmd` (`@"%~dp0OmarchyOverlay.exe" --install`); README "Releases" section.

- `python3 scripts/package.py [--exe PATH] [--out dist]` builds from the release exe (default `target/x86_64-pc-windows-msvc/release/OmarchyOverlay.exe`):
  - `dist/OmarchyOverlay-<version>-standard.zip`: `OmarchyOverlay.exe`, `RECOVER.cmd`, `fonts\` (from `assets/fonts`), `config\examples\`, `README.txt` (Standard section). No `INSTALL.cmd`, no marker.
  - `dist/OmarchyOverlay-<version>-admin.zip`: the same files plus `INSTALL.cmd` and an empty `edition-admin`, `README.txt` (Admin section).
  - `<version>` read from `Cargo.toml`; both zips' exe are byte-identical (assert by hash).
- `--selftest`: builds both zips from a dummy exe in a temp dir and asserts the file lists above and the identical exe hash.
- [ ] Steps: selftest → fail → implement → pass → commit.

### Task 13: Stage 3 VM evidence, both editions

**Files:** Create `tests/windows-install.ps1`, `tests/windows-standard.ps1`; add `install` and `standard` to `scripts/test_windows.py` (`standard` pushes and unzips the Standard zip; `install` the Admin zip).

- [ ] **Step 1: Standard edition as `worktest`** (signed in at the console; `OVERLAY_VM_USER=worktest`, guest `C:\Users\worktest\Overlay-Test`): unzip, start the exe; assert the process token is not elevated and the owner is worktest, no `consent.exe` appeared, the tray has no install item, Win key binds that need elevation are reported as unavailable rather than failing, exit restores the desktop, and nothing was written to `Program Files`, `HKLM\...\Uninstall\OmarchyOverlay`, `%ProgramData%` Start menu or Task Scheduler. Then run `smoke customization appearance-integration` as worktest; expect `All requested native suites passed.`
- [ ] **Step 2: Admin edition as `Matt`** (admin; accepts UAC by running elevated through the guest agent): `INSTALL.cmd` → assert files, task (`schtasks /Query /TN "Omarchy Overlay"`), shortcut, uninstall key; `schtasks /Run` → app running elevated; open an elevated `notepad` and send a bound Super key with `SendInput` → assert the bound command ran (log line); trigger the terminal command → assert the new process's TokenElevation is 0; `--uninstall` → assert none of the above remain and the recovery checks from Task 4 pass.
- [ ] **Step 3:** Run the full default suite list plus `install` as Matt, and `standard` as worktest; expect `All requested native suites passed.` for each.
- [ ] **Step 4:** README: the two editions and which to pick; remove the "administrator windows do not pass keys" limitation for Admin only; document install/uninstall, Alt binds; CHANGELOG; commit.
