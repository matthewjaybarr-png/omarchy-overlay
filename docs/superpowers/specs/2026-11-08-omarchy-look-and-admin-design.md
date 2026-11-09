# Omarchy look, themes and admin mode — design

Date: 2026-11-08. Status: approved in conversation, awaiting spec review.

## Goal

Make Omarchy Overlay look and feel as close to Omarchy/Hyprland as Windows 11
allows, pick up the Omarchy themes, and ship two editions as separate
releases: a portable, per-user **Standard** edition that never needs admin, and
an installed **Admin** edition that runs elevated so keys work over elevated
windows. Hard rule: normal Windows behavior never breaks, and
exit, crash recovery and `RECOVER.cmd` always restore the desktop.

Built in three stages, each committed on `main` with passing `cargo test`, clean
`cargo clippy --target x86_64-pc-windows-msvc -- -D warnings`, and evidence from
the libvirt "Windows 11" VM (`scripts/test_windows.py`).

## Stage 1 — Themes

**Sources.** `/usr/share/omarchy/themes/*` (stock) and `~/.config/omarchy/themes/*`
(custom) on the Linux laptop.

**Allow rules (in the script, not config):**
- Skip themes named `chatgpt-image-*`, `wallhaven-*` and `matt1`.
- `neon-ridge`: drop wallpapers `01-*`, `02-*`, `03-*` (they show branding icons);
  strip comment lines from its `colors.toml`.
- Any theme without a valid `colors.toml` is skipped with a message.

**`scripts/import_omarchy_themes.py`:**
- Default: writes each allowed theme's `colors.toml` to `themes/omarchy/<id>.toml`
  in the repo (comments stripped). These are embedded in the exe with
  `include_str!` via a generated list, so every install has them.
- `--out <dir>`: also builds `<dir>/<id>/colors.toml` + `<dir>/<id>/backgrounds/`
  (allowed wallpapers only) for copying to a PC or VM. Wallpapers never enter
  the repo.

**Runtime.** Themes resolve in this order: user folder
`%LOCALAPPDATA%\OmarchyOverlay\themes\<id>\colors.toml` (Omarchy layout, with
optional `backgrounds\`), then legacy `themes\<id>.toml` files, then embedded.
The hard-coded palettes in `src/theme.rs` are removed; Neon Ridge stays the
default and fallback, now from its embedded file.

**Parsing.** `src/theme_files.rs` keeps every Omarchy key: background
variants (`dark_`, `darker_`, `lighter_background`), foreground variants,
`accent`, `selection`, `muted`, the 8 normal + 6 bright terminal colors, and
`mode`. Existing derived fields (surface, border, muted) keep their current
fallback rules. Unknown keys are ignored, not errors, so new Omarchy keys
don't break loading. Bad values still name file and line.

**Tests.** Unit tests parse every embedded file; the script has an
assert-based self-check for the skip/ban rules.

## Stage 2 — Look

**Bar (Waybar layout).** JetBrains Mono Nerd Font (bundled in `assets/fonts`).
Workspaces left (active in accent), clock center, tray/network/battery/volume
right, theme background. Built on the existing `src/bar.rs` modules.

**Borders and gaps (Hyprland).** Focused window: 2 px accent border; others:
muted border; square corners; gaps 5 inner / 10 outer. Uses Windows 11 DWM
attributes (`DWMWA_BORDER_COLOR`, `DWMWA_WINDOW_CORNER_PREFERENCE`). Before
changing a window, its prior values are recorded; exit, recovery and
`RECOVER.cmd` reset `DWMWA_BORDER_COLOR` to default and corners to default on
every window the app touched. The existing click-through `focus_outline.rs`
remains the fallback where DWM refuses (e.g. Windows 10, elevated windows when
not elevated).

**Launcher and menus (Walker).** Centered, theme colors and font, accent
selection highlight, Omarchy-style menu keys.

**Theme drives Windows.** Switching theme sets the wallpaper (cycled from the
theme's `backgrounds`), Windows accent color and app/system dark-light mode,
like `omarchy-theme-set`. Prior values are saved on first change and restored
on uninstall and from the recovery menu.

**Tests.** VM screenshots per theme switch (bar, menu, border), recovery test
proving borders/corners/wallpaper/accent reset.

## Stage 3 — Two editions, admin installer and restrictions

**Two editions, one exe.** Both releases ship the same `OmarchyOverlay.exe`,
built once, so every suite covers both. There is no cargo feature: the edition
is decided at run time.
- **Standard** (`OmarchyOverlay-<version>-standard.zip`): portable, per-user,
  no installer, never asks for UAC. Runs from any folder (Downloads, a stick, a
  user folder) as a standard user. Settings go in `%LOCALAPPDATA%\OmarchyOverlay`, or `OVERLAY SETTINGS`
  beside the exe when that folder exists. Its tray menu and docs have no install
  entry; elevated windows stay out of reach (the Windows limit, documented).
- **Admin** (`OmarchyOverlay-<version>-admin.zip`): the same exe plus an
  `INSTALL.cmd` that runs `OmarchyOverlay.exe --install`. Once installed it runs
  elevated from the logon task. A marker file `edition-admin` beside the exe in
  the zip turns on the tray's "Install (admin)…" item; the Standard zip lacks it.
- What actually differs is decided by facts, not by the zip: elevated-only
  behavior (Super keys, keys over elevated windows, de-elevated launches)
  follows `process_elevated()`, and installed-only behavior (uninstall item,
  `%LOCALAPPDATA%` settings) follows whether the exe sits in the install dir.
  So the Standard zip run by an admin simply behaves as Standard, unelevated.
- The `--install` / `--uninstall` switches work from either zip; the marker
  only decides what the tray offers.


**Installer in the exe.** `OmarchyOverlay.exe --install` / `--uninstall`, also
in the tray menu. Relaunches itself elevated (UAC once), then:
- copies the exe and theme/font files to `%ProgramFiles%\OmarchyOverlay`;
- creates a Task Scheduler logon task "run with highest privileges" for the user;
- adds a Start menu shortcut and an Apps & features uninstall entry.
`--uninstall` stops the app, restores the desktop and Windows settings, and
removes all of the above. User settings in `%LOCALAPPDATA%` are kept unless
the user ticks "remove settings". The portable zip still runs unelevated.

**De-elevated launches.** When elevated, programs the app starts (launcher,
terminal, web apps) are started through Explorer's token (e.g. via the shell
`IShellDispatch2::ShellExecute` on the desktop's Explorer), so they don't run
as administrator.

**Restrictions removed:**
- Plain Alt binds: allowed with a warning (`src/binds.rs`).
- Settings folder: installed copy uses `%LOCALAPPDATA%\OmarchyOverlay`; a
  portable copy may still use `OVERLAY SETTINGS` beside the exe, not required
  to be on a stick (`src/profile.rs`).
- Super keys: when elevated, the low-level hook may take Super+Q, Super+Return,
  Super+Space and similar. Ctrl+Alt+Del and Super+L always stay with Windows.

**Restrictions in each edition.** The plain-Alt removal and the
optional settings folder apply to both editions. Super-key capture needs
elevation, so in practice only the installed Admin edition gets it.

**Tests.** VM, Standard edition as `worktest` (a standard user): unzip to a
user folder, run, smoke and customization suites pass, no UAC prompt, the
process is not elevated, no install entry in the tray, nothing written outside
the user profile and the unzip folder. VM, Admin edition as `Matt`: install, keys reach the app over an elevated window, launched
programs are not elevated, uninstall leaves no files, task, shortcut or
registry entry, and the desktop is restored.

## Out of scope

Bundling stock wallpapers; uiAccess/code signing; an Inno Setup wizard;
Windows 10 visual parity.
