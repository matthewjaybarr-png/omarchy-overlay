# Changelog

## Unreleased

- Menus follow Omarchy's menu card: square corners, a 2 px border, a faded
  search line, the accent color on the selected row and Omarchy's sizes.
- The app picker is a grid like Omarchy's launcher: large app icons with their
  names, 7 across and 4 rows at a time, moved with the arrow keys.
- The README shows screenshots of the menu, the app grid and the Tokyo Night look.

## 0.3.0

- Tokyo Night, Omarchy's default, is the theme of a new setup. Settings you already
  have keep their theme.
- The Admin edition ships Omarchy's backgrounds for every theme; switching theme sets
  its first background and the background switcher lists them.
- More stock Omarchy keys: **Super+Shift+N** editor, **Super+Shift+Q** calculator,
  **Super+Shift+M** music, **Super+Shift+T** Task Manager, **Super+Alt+F** full width.
- **Appearance > Top bar > Icons or text** switches the whole bar between icons,
  icons and text, and text, or sets each item on its own. Each module under
  **Bar items** has the same choice.
- **Restore defaults** on **Bar items** puts the default layout back.
  **Reset the top bar** and **Setup > Reset all settings** reset from the menu, after
  asking; a full reset keeps saved profiles and backs up the old settings.
  **Restore defaults** in the bar and module windows now applies at once.
- Security: the installed Admin edition no longer looks for an `OVERLAY SETTINGS`
  folder above `%ProgramFiles%`, where any user could create one. Its settings and
  config files are in `%LOCALAPPDATA%\OmarchyOverlay`, editable without admin rights.
- The Admin installer updates in place: it detects the installed version, updates an
  older one (asking before a downgrade), stages and verifies the new files before
  swapping them in, rolls back if anything fails, removes files a release no longer
  ships, waits for the old copy and its recovery guard to exit, and deletes files
  still in use at the next restart. The tray offers **Update the installed copy**,
  `--quiet` installs and uninstalls without messages, the exit code reaches
  `INSTALL.cmd`, only one installer runs at a time, and the Apps entry has the size,
  install date and a quiet uninstall command. Uninstall retries removing the folder.
- Restart keeps arguments that end in a backslash.
- The About box describes the installed edition correctly; the release zips include
  the licence; CI runs the tests, Windows clippy and the package self-test.

## 0.2.0

- Every Omarchy theme is built in, Neon Ridge first, replacing the old
  hand-written palettes. A theme you add under `config\themes\` as a
  `<name>.toml` file or an Omarchy theme folder (`<name>\colors.toml`, with
  `backgrounds`) loads too, and replaces a built-in theme of the same name.
- Unknown keys in a `colors.toml` are ignored instead of rejecting the file.
- Omarchy look by default: Waybar-style bar, square 2 px borders in the theme's
  active and inactive colors, and Walker-style menus in JetBrainsMono Nerd Font.
- New bars show icons for every item except the clock (and window title), using
  Omarchy's own bar glyphs for tray, sound, network, battery, tiling,
  notifications, clipboard and capture. Saved profiles keep their labels.
- The theme sets the Windows accent color and its first background at start
  and when chosen; `next-background` cycles the theme's
  backgrounds. Both are restored on exit and after a crash.
- Two editions from one exe. **Standard** is portable and never asks for
  administrator rights. **Admin** adds **Install (admin)…** to the tray (or
  `INSTALL.cmd`): it installs to `%ProgramFiles%\OmarchyOverlay` with an elevated
  logon task, a Start menu shortcut and a Settings > Apps entry, so its keys work
  in apps running as administrator. Apps it starts still run unelevated.
  **Uninstall…** restores the desktop and removes all of it.
- No app is special-cased any more, plain Alt binds are allowed with a warning,
  and profiles are no longer looked up only on removable drives.
- `scripts/package.py` builds the Standard and Admin release zips.
- The built-in keys now follow stock Omarchy: Super+Space is the menu and
  Super+Alt+Space the apps; Super+W or Super+Q closes, Super+M minimizes,
  Super+Ctrl+Tab returns to the former workspace, Super+Shift+Space toggles the
  bar and Super+Ctrl+Space opens the background switcher. New universal
  clipboard commands (`copy`, `paste`, `cut`, `select-all`) sit on
  Super+C/V/X/A, with Ctrl+Shift+C/V in Windows Terminal. Omarchy's web apps
  (ChatGPT, Grok, WhatsApp, YouTube, Google Photos, Google Maps, X) get their
  usual Super+Shift keys. Personal extras such as Super+Z, Super+B and Super+E
  are gone. An existing `binds.conf` is kept as it is.

## 0.1.0

- The first public version. Omarchy Overlay starts from a personal Windows
  window manager, renamed so the two never share an executable, window classes,
  instance lock or `%LOCALAPPDATA%` folder.
