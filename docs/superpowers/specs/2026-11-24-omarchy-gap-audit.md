# Omarchy gap audit

Date: 2026-11-24. Compared Omarchy at commit `7427fab` (basecamp/omarchy,
2026-11-24) against this repository at `459189b`. Feeds Phase 1 (Standard) and
Phase 2 (Admin) of `2026-11-24-full-windows-theming-design.md`.

Each Omarchy feature has an overlay status (**Done**, **Partial**, **Missing**) and
a Windows feasibility:

- **Standard:** works unelevated. Super keys come through the keyboard hook, but
  not while an elevated window is in front.
- **Admin:** needs the elevated edition.
- **Add-on:** needs an invasive third-party engine.
- **Not possible.**

Omarchy file references are relative to the Omarchy repository. "Unverified" means
not yet checked in code or on Windows.

## Summary

Most of Omarchy's tiling keys, window states, groups, workspaces, monitors, gaps,
borders, themes, accent and dark mode are already done. The biggest gaps:

1. **Floating workspace mode:** none today.
2. **A dock:** none today. In Omarchy the two are linked: the dock shows only on
   floating workspaces, and Super+M minimizes to it.
3. **Theme fidelity outside the overlay's own windows:**
   - Windows Terminal gets Tokyo Night ANSI colors for every theme.
   - The Standard zip has no backgrounds.
   - VS Code and Win32 system colors aren't themed.
   - A single font choice doesn't reach the other apps.
4. **Shell polish:** the volume and brightness popup is a plain text label, the bar
   is top-only, and there's no color picker, idle screensaver or attention
   highlight for apps that flash.

## Progress (2026-11-25, evening)

Built on `forge/quirky-dirac-tx8mpd` (PR #32). Unit tests, Windows clippy and the 23
VM suites pass (VM at 423d207); builds 7–9 tested on a Windows 11 VM through
`docs/testing` (work branch only).

- P1-A: done — floating layout, per-workspace layout keys, gap toggle (edges too).
- P1-B: dock done (pins, running apps, cycling, middle/right click, auto-hide mode,
  attention ring, stable order, clear of an auto-hidden taskbar). Open: hidden
  taskbar as the default, after the Start / Win+N / Win+A / toast / tray checks.
- P1-C: done — Terminal ANSI and black/white, VS Code, accent palette, classic
  selection colors, swatches, transparency modes, one font, per-user font install,
  colors back at every start, backgrounds in both zips. Open: git theme install,
  Obsidian/Neovim, night light.
- P1-D: done — OSD card, color picker, idle screensaver and lock, persistent
  reminders, status indicators, bar at top or bottom, defaults merged into users'
  config files, full-screen desktop, hibernate, voice typing. Open: left/right bar,
  clickable calendar and audio panels, terminal-app launcher, welcome tour.
- P1-E: done — multi-window scratchpad, drop-down console, per-monitor workspaces
  (setting, off by default), group tabs, group window keys, close-all. Open:
  pseudo-tile, square aspect toggle, laptop display toggles, window animations.

## Phase 1 — Standard edition, in build order

### P1-A. Floating workspace mode and per-workspace layouts

Omarchy's behavior (`default/hypr/workspace-modes.lua`, `workspace-layouts.lua`,
`bin/omarchy-hyprland-workspace-layout-toggle`):

- **Cycling:** Super+L / Super+Alt+L cycle the active workspace through dwindle,
  scrolling and floating. The overlay uses Super+Shift+L, because Super+L is
  Windows' lock key.
- **Per workspace:** the mode belongs to one workspace and is saved across restarts.
  A global default exists too; changing it freezes busy workspaces at their
  current layout.
- **Entering floating:** every tiled window is tagged and floated at
  min(1100, 64% of width) × min(760, 68% of height). Windows are centered, each
  offset 24 px from the one before (up to 5 steps). New windows on that workspace
  open floating the same way.
- **Moving windows across workspaces:** a window moved onto a floating workspace
  floats. A tagged window moved to a tiled workspace re-tiles.
- **Leaving floating:** only tagged windows re-tile. Windows floating for another
  reason stay floating.
- **While floating:** click-to-focus on that workspace, the dock is shown, and
  Super+M minimizes to the dock.

Tasks:

1. Add `floating` as a layout in `workspace_layouts.rs` with the placement above,
   as a pure function with unit tests. Track the tag in the window state.
2. Make `set_layout` always write the current workspace's choice
   (`desktop_actions.rs:114-150`). Today it falls back to changing the global
   layout when the workspace has no choice of its own. Add the "freeze busy
   workspaces" step when the default changes.
3. Restart, crash and `RECOVER.cmd` behave as for the other layouts.
4. Quick key fixes from the same audit:
   - Super+Shift+Backspace runs `toggle-gaps` (today it opens the gaps page).
   - Add a combined bar-off-and-no-gaps command on Super+Ctrl+Alt+F.
   - Bind group focus to Super+Ctrl+Left/Right.
   - Add group member 1–5 on Super+Alt+1..5.

### P1-B. Dock, then the taskbar hidden by default

Omarchy's dock (`shell/plugins/dock/`):

- **Contents:** pinned apps in pin order, then running apps grouped by app.
- **Clicks:**
  - Left click brings the app's window forward. It picks a window on this
    workspace first, then a minimized one, then one elsewhere, and repeated
    clicks cycle through them.
  - Middle click opens a new window.
  - Right click offers New window, Keep in Dock / Remove and Quit.
- **Pinning:** drag to reorder, and drag from the app grid to pin.
- **Display:** autohide, top/bottom/left/right, one dock per screen, a launch
  pulse until the window appears, theme colors.

Tasks:

1. Build a dock window per monitor. Reuse the bar's per-monitor panels
   (`bar.rs`) and the AppUserModelID grouping in `tasks.rs`. Pins come from the
   picker favorites.
2. Launch pins through `shell:AppsFolder\<AUMID>` or the `.lnk` file. Go through
   `launch_unelevated.rs` when the overlay is elevated.
3. When to show it:
   - Omarchy's rule: on floating workspaces.
   - Option: always, when the taskbar is `hidden`.
   - Hide it behind fullscreen windows.
4. **Attention:** register a shell hook (`RegisterShellHookWindow`,
   `HSHELL_FLASH`) and highlight the dock icon, the task button and the workspace
   number. The overlay only sends flashes today and never listens for them.
5. Minimized windows must stay reachable from something visible (the dock) once
   the taskbar is hidden.
6. Verify in the VM, with the taskbar in `hidden` mode, that these still work:
   Start, Win+N, Win+A, toasts and the tray (unverified today). Then make
   `hidden` the default taskbar mode.
7. A quick first step, if wanted: show pins in the bar's Tasks module.

### P1-C. Theme fidelity

1. **Terminal ANSI colors.** The Windows Terminal fragment should use the theme's
   `color0`–`color15`. Today `appearance_profiles.rs:114-125` hard-codes Tokyo
   Night for every theme except Neon Ridge. Small change, very visible.
2. **Backgrounds in the Standard edition.** Today only the Admin zip ships stock
   backgrounds. Decide between shipping them in the Standard zip and downloading
   them on first use.
3. **One font for everything:** the overlay UI, bar, Terminal fragment `font.face`,
   VS Code editor and terminal font, and title-bar/menu fonts
   (`SPI_SETNONCLIENTMETRICS`). This mirrors `omarchy-font-set`. Optionally install
   the Nerd Font per user.
4. **VS Code family** (Code, Insiders, VSCodium, Cursor):
   - Use the theme's `vscode.json` extension name if it has one.
   - Otherwise generate `workbench.colorCustomizations`.
   - Journal the `settings.json` keys.
   - Installing a marketplace extension stays opt-in.
5. **Win32 system colors and `AccentPalette`** from `colors.toml`, re-applied at logon.
6. **Theme picker previews:** swatches from `colors.toml`, or `preview.png`.
   Install, update and remove themes from a git URL, keeping only colors and
   backgrounds, following Omarchy's rule against code-bearing files.
7. **Transparency modes:** opaque / transparent / glassy as one setting, mapped to
   opacity plus Mica/Acrylic (`omarchy-hyprland-window-transparency`).
8. **Smaller items:**
   - Obsidian vault theme and Neovim colors.
   - A screensaver registered per user (HKCU `SCRNSAVE.EXE`).
   - Night light.
   - Themed app-icon set for the dock and launcher (Omarchy `icons/` recolored).

### P1-D. Shell polish

1. **Volume, brightness and workspace popup:** icon, progress bar and theme colors
   (`plugins/osd/Osd.qml`), replacing today's text label.
2. **Bar position** (bottom, left, right) and opaque/glassy/clear backgrounds.
3. **Bar indicators:** stay awake, pending reminder, mic mute, keyboard layout,
   media.
4. **Color picker** on Super+Print.
5. **Idle:** screensaver after 150 s and lock after 300 s by default
   (`GetLastInputInfo`), with settings.
6. **Reminders** with a custom message that survive a restart.
7. **Clickable panels** for clock/calendar and audio, instead of jumping straight
   to Windows' flyouts.
8. **Smaller items:**
   - Voice typing (Win+H) command; hibernate in the power menu.
   - A terminal-app launcher template.
   - Conditional and checked rows in `menu.json`.
   - A welcome tour.

### P1-E. Workspaces and groups

1. **Scratchpad that holds several windows**, shown over the current workspace.
2. **Quake-style drop-down console:** Super+\` / Super+Shift+\`
   (`default/hypr/qconsole.lua`).
3. **Per-monitor workspaces:** each monitor keeps its own 1–10. At least offer an
   option to switch only the focused monitor.
4. **Group tab bar** over a group, plus the group member on Super+Alt+wheel.
5. **Smaller items:**
   - Pseudo-tile (Super+P).
   - Single-window square aspect toggle (Super+Ctrl+Backspace).
   - Close all windows (on a free key; Omarchy uses Ctrl+Alt+Delete, which Windows
     reserves).
   - Laptop display off and mirror.
   - Window open/close animations.
   - Key repeat and numlock settings.

## Phase 2 — Admin edition

- Keys and DWM frame colors over elevated windows (UIPI behavior to verify in the VM).
- The dock closing and minimizing elevated windows.
- Browser theme color through the `BrowserThemeColor` / `BrowserColorScheme`
  policies (`omarchy-theme-set-browser`). Chrome supports them; Edge is unverified.
- Sign-in screen colors and image (Omarchy themes its login screen), machine-wide
  lock screen image.
- Reboot to firmware (BIOS) in the power menu.
- Monitor scaling keys (Super+/ and Super+Alt+/), through an undocumented
  DisplayConfig call (also possible unelevated; to check).

## Not possible on Windows (documented, not planned)

| Omarchy feature | Why |
|---|---|
| Themed lock screen and fingerprint UI | The lock screen runs on Windows' secure desktop |
| Own notification daemon, do-not-disturb, dismissing other apps' toasts | No public API without package identity; Settings deep link only |
| Jump lists, taskbar progress and badges in the dock | Data goes only to Explorer, through undocumented storage |
| Boot splash (Plymouth) | Firmware/OEM territory |
| Compose key (Shift+Caps) | Needs a third-party IME |
| Tiled fullscreen (app thinks it's fullscreen inside its tile) | No such window state on Windows; F11 per app at best |
| Themed input method (fcitx5) | Windows IME UI can't be themed |
| Gradient window borders | DWM borders are one color; possible only by drawing our own outline |
| Real blur behind app contents | Only DWM backdrops on frames; more needs an add-on |
| Keyboard RGB | Dynamic Lighting rules unverified; add-on at best |

## To verify

- With the taskbar hidden: Start, Win+N, Win+A, toasts and the tray all still work.
- A Standard process bringing forward elevated windows (expected to work) and
  closing them (expected to be blocked).
- That the dwindle split matches Omarchy's `force_split=2`; scrolling column width
  0.49.
- Whether popped windows re-center on another monitor, and whether the cursor
  follows focus on a workspace switch.
- Edge and `BrowserThemeColor`; the media session API from an unpackaged exe;
  lock-screen WinRT from an unpackaged exe.
