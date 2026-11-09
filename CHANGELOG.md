# Changelog

## 0.2.0 (in progress)

- Every Omarchy theme is built in, Neon Ridge first, replacing the old
  hand-written palettes. A theme you add under `config\themes\` as a
  `<name>.toml` file or an Omarchy theme folder (`<name>\colors.toml`, with
  `backgrounds`) loads too, and replaces a built-in theme of the same name.
- Unknown keys in a `colors.toml` are ignored instead of rejecting the file.
- Omarchy look by default: Waybar-style bar, square 2 px borders in the theme's
  active and inactive colors, and Walker-style menus in JetBrainsMono Nerd Font.
- The theme sets the Windows accent color and its first background at start
  and when chosen; `next-background` (Super+Ctrl+Space) cycles the theme's
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

## 0.1.0

- The first public version. Omarchy Overlay starts from a personal Windows
  window manager, renamed so the two never share an executable, window classes,
  instance lock or `%LOCALAPPDATA%` folder.
