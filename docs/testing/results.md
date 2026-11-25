# Test results

Newest first. Written by the Windows session; see README.md.

## Build 10 (ddcd9bd) — 2026-11-25 16:46 UTC, Windows 11 VM

Standard zip runs portable from `C:\Users\Public\ovt\b10\std`. One 1920x1080 monitor.
Screenshots in `shots/10-*.png`.

- PASS BG1: both zips have `themes/<id>/backgrounds` (gruvbox has 6 jpgs).
- PASS K1 full-screen desktop (`10-K1-full`): Super+Ctrl+Alt+F hides the bar and the
  gaps, windows reach the screen edges; pressing it again brings both back.
- PASS K1 group keys: Super+G, then Super+Alt+Up moves a window into the group;
  Super+Alt+1 / Super+Alt+2 switch the active tab. The tab strip shows both windows.
- PASS K1 voice typing (`10-K1-voice`): Trigger → Voice typing opens the Windows
  dictation popup.
- PASS K1 hibernate (`10-K1-hibernate`): System → Hibernate asks "Hibernate?" first;
  chose No, nothing happened.
- Not tested K1 close-all: build 10 has no way to send a command from the command line
  (the exe only takes --restart, --install, --uninstall, --check-profile, --watchdog)
  and no menu entry for it. Will try it as a typed command in build 11 (O6). How is it
  meant to be run?
- Side note: one restored Notepad stayed floating in front while tiling was on. Not
  investigated; may be a window Notepad restored before tiling started.

## Build 9 (4c2a8d5) — 2026-11-25 16:45 UTC, Windows 11 VM

Standard and Admin zips run portable from `C:\Users\Public\ovt\b9`; Admin run
elevated. One 1920x1080 monitor, then two 1280x800 monitors for W1. Screenshots in
`shots/9-*.png`.

- PASS log at start: no `Font ""` line; tray lookup "selected SystemTray.NormalButton /
  Tray preparation: ready".
- PASS B1 (`9-B1`, `9-B1-tip`): mic off shows the mic-off icon, stay awake shows the cup.
  With both on only one icon shows (mic-off wins); the tooltip lists both. Is one icon
  at a time intended?
- PASS D8 (`9-D8`): "Dock always (auto-hides)" with the taskbar on auto-hide: pointer at
  the bottom shows the dock above the taskbar; both hide when the pointer leaves.
- PASS TRAY (`9-TRAY-std`): Standard and elevated Admin, the chevron opens the Windows
  overflow, no card, no freeze; the tooltip fits on one line. The card-width and
  tooltip-wrap fixes were not exercised because the lookup no longer fails.
- Labels (`9-labels-trigger`, `9-labels-taskbar`): Trigger and Toggle fit. Still cut:
  "Hide the Windows taskbar while Omarchy Overla…" (Appearance → Windows taskbar) and
  "Each monitor has its own workspaces (as in …" (Layout, `9-W1-label`).
- PASS P1 (`9-P1`): bar moves to the bottom, survives a restart, back to top shows
  "Saved.". Once, after the menu closed, the Windows taskbar showed over the bottom bar
  (during a Standard/Admin swap); not reproduced on a recheck. Not checked: tiles and
  dock with the bar at the bottom (tiling was paused).
- Main menu opens by itself at start (`9-menu-at-start`): seen twice with elevated
  Admin and once with Standard on two monitors.
- W1, two monitors (`9-W1-a2`): mostly PASS.
  - Super+2 on A hid only A's 5 windows; B's 2 stayed. Super+1 on A brought them back.
  - Super+Shift+3 on B sent the window to B's workspace 3; B's other window stayed.
  - Turning the setting off brought B's workspace-3 window back.
  - "Restore desktop and exit" restored all 7 Notepads, no process left.
  - FAIL bar: both monitors' bars show the workspace of the monitor last switched.
    After Super+2 on A, B's bar also shows 2, and still did after focusing B with
    Ctrl+Alt+Tab. (The VM pointer cannot reach monitor 2, so I could not click a window
    on B; focus came from Ctrl+Alt+Tab.)
  - A Photos window that was open on A, untiled, appeared when A switched to 2.

## Build 7 (2c3f776) — 2026-11-25 15:20 UTC, Windows 11 VM, 1 monitor at 125%

Standard zip, run portable from `C:\Users\Public\ovt\b7\std` as a standard user, with a
user `config\binds.conf` and `menu.json` seeded from build 6's examples (one line edited).
Theme Gruvbox. Screenshots in `shots/7-*.png`.

- PASS M1: new keys added at the end under "# Added by Omarchy Overlay", my edited line
  kept, `binds.conf.before-0.5.0` and the menu.json backup written, Appearance gained VS
  Code colors, Window transparency and Screensaver and idle lock. Not checked: whether
  unedited lines follow changed defaults (none of the seeded lines changed between 6 and 7).
- PASS F7 (`7-F7`): first press shows "Gaps off" and takes away edge and inner gaps;
  second shows "Gaps on" and puts them back. No opacity change.
- PASS C1, S1, G1 (`7-G1`: tab strip with 2 tabs, "Group 2/2").
- PASS T1 (`7-T1`): red is #EA6962, background #282828, but only after Terminal is
  restarted; a Terminal already open when Add is clicked keeps Campbell (#E74856).
- PASS R1 / exit: wallpaper set to a theme background, then Update → Process → Restore
  desktop and exit. Wallpaper back to `img19.jpg`, taskbar and windows restored, no
  process left. Windows reported the real `img19.jpg` path, not `TranscodedWallpaper`,
  so the new `wallpaper-original.jpg` copy was not exercised.
- FAIL B1 icon: tooltip is right (`7-B1`: "Stay awake is on / Microphone is off") but the
  icon stays the keyboard glyph. Keyboard-layout part untested (one layout).
- FAIL D8 (`7-D8`): "Dock always (auto-hide)" with the Windows taskbar on auto-hide:
  pointer at the bottom shows both, and the dock tile overlaps the taskbar instead of
  sitting above it. Both hide when the pointer leaves. "No dock" shows only the taskbar.
- TRAY (Standard, `7-TRAY`): card "…tray could n…" (cut off) and a tooltip "Tray control
  lookup failed (0x80004005)…" that runs off the right edge. No freeze. Admin not tested.
- Labels cut off in many menus (`7-labels`): "Layouts by works…", "Screensaver and …",
  "Choose a section, …", "Left …", "Right…", "Running apps and t…", "Clipboard h…",
  "Screen reco…", "Add the …", "Open Windows Termi…", "Run Add again afte…".
- Log at start: `Font "" is not available; Segoe UI is used instead`.

## Build 6 (02242fa) — 2026-11-25, Windows 11 VM (partial, superseded by build 7)

- The Actions exe starts: the manifest fix works.
- PASS F7: toggled the inner gap 0 ↔ 5 only (edge gap unchanged), as the cloud found.

## Pre-build 5 (f5ed887) — 2026-11-25, Windows 11 VM, 1 monitor at 125%

Tested before Build 5 existed, from the Standard and Admin zips with the exe built
locally at f5ed887. Standard as a standard user; Admin installed with INSTALL.cmd for an
administrator (logon task, elevated). No screenshots: the VM desktop shows private items.

**Blocker for every Actions build: the exe won't start.** lld-link 18 on the Ubuntu
runner merges a second trustInfo into the manifest and writes `ms_asmv1:level`, which
Windows refuses to load. Fixed on main by matthewjaybarr-png/omarchy-overlay#31
(`build.rs`: `println!("cargo:rustc-link-arg=/MANIFESTUAC:NO");`). This branch needs it
too (merge main or cherry-pick), or no Actions artifact can be tested.

- PASS F2, F3, F4, D1, D2, D3, D4, D6, D7, D8, D9, D10, T1, F8 (Standard and Admin)
- FAIL F1: centred and staggered 24 px, but about 57% of the screen wide, not two-thirds.
- FAIL F7: Super+Shift+Backspace is still bound to restoring window opacity; gaps don't toggle.
- PASS F5 (re-tiles when cycling back); the Super+T part was not tested.
- PASS F6 (cycles backwards); "workspace 2 unchanged" was not checked.
- PASS D5, but Keep in dock is greyed out for apps with no picker entry (Dropbox).
- T1 note: the fragment's black, brightBlack and brightWhite are Tokyo Night constants
  (`appearance_profiles.rs` ~148), not theme colours.
- Dock: unpinned icons reorder by z-order on every focus change.
- Dock: with the taskbar on auto-hide, the popped-up taskbar overlaps the dock, or the
  dock covers its Search box.
- Menus: labels are cut off in the dock options and "Apply theme to Windows"
  (e.g. "Windows d…", "Acc…").
- F8 note: after the Standard exit the wallpaper was black; the original wasn't checked.
- **Admin bug: a hidden message box freezes the overlay.** Clicking the bar's TRAY button
  fails ("Tray control lookup failed (0x80004005)…"; log: "No supported hidden-icons
  control was found; SystemTrayIcon classes: []") and shows a message box *behind* the
  other windows. Until its OK is clicked, keys, tiling and the bar clock all stop.
  Suggest a non-modal toast/OSD instead, or at least MB_SETFOREGROUND|MB_TOPMOST.
- Admin: keys work while an elevated cmd window is focused. Elevated windows are not
  tiled (the `elevated()` filter); fine if intended, but the README only says "keys".

