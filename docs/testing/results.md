# Test results

Newest first. Written by the Windows session; see README.md.

## Build 7 (2c3f776) — 2026-11-25 10:30 VM time, Windows 11 VM, 1 monitor at 125%

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

