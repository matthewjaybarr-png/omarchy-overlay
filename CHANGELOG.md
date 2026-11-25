# Changelog

## Unreleased

- **Typed commands in the menu:** type any binds.conf command into the menu's search
  and press Enter, as in Omarchy, for example `remind 25 Stretch`.
- **Install themes from git**, as omarchy-theme-install: `theme-install owner/repo`
  (GitHub) or an https:// git address downloads the theme with git, keeps only its
  colors.toml and background pictures in `config\themes\<id>`, and switches to it.
  Nothing that runs code is copied. Needs Git for Windows.
- **Super + P** is Omarchy's pseudo-tile: the focused window keeps its own size,
  centred in its tile, until you press it again. It takes Windows' Super + P (Project);
  display projection stays in Settings > Display.
- **Super + Ctrl + Backspace** is Omarchy's single-window aspect ratio: a workspace's
  only tiled window stays a centred square instead of stretching across a wide screen.
  Press it again to let it fill the screen.
- More of Omarchy's keys and commands: **Super + Ctrl + Alt + F** hides the bar and
  the gaps for a full-screen desktop, and brings them back; **Super + Alt + 1–5**
  shows that window of the focused group; `close-all` closes every app window on this
  workspace (each app can still ask about unsaved work); **Hibernate** in the System
  menu; **Voice typing** in the Trigger menu opens Windows dictation.
- The Standard zip carries Omarchy's stock theme backgrounds too, so every theme sets
  its own wallpaper in both editions.
- **Per-monitor workspaces** (Layout > "Each monitor has its own workspaces"), off by
  default: as in Omarchy, switching workspace changes only the monitor in use, and
  moving to another monitor shows its workspace. Layout choices follow each monitor's
  own workspace. Turning it on or off brings every hidden window back first. The bar
  shows the workspace of the monitor in use on every monitor.
- **Bar position** (Appearance > Top bar > Bar position): the bar can sit at the
  bottom of the screen, as Omarchy's can. Windows and the dock stay clear of it.
- Exit puts back the stock or Settings-chosen wallpaper instead of leaving the
  desktop black: Windows reports its own cache file as the wallpaper and overwrites
  it with the next one, so a copy is kept before the first change.
- **Super + Shift + Backspace** takes away the gaps at the screen edges too, as
  Omarchy's toggle does, and says "Gaps off" / "Gaps on".
- **Group tabs**, Omarchy's groupbar: a grouped tile gives up a 24 px row at its top
  for a tab per window, the shown one in the accent color. Clicking a tab shows that
  window.
- **New defaults reach your own config files.** When a version adds keys to
  binds.conf or entries to menu.json, they are merged into your copies at start: new
  keys and entries are added, lines you never changed follow the new defaults, and
  anything you changed or deleted stays as it is. Your file from before is kept as
  `binds.conf.before-<version>` / `menu.json.before-<version>`.
- Fixes from the build 7 test run: the status indicators icon follows the most urgent
  indicator; the dock sits above the taskbar whenever Windows auto-hides it, whichever
  setting does that; the on-screen card widens to fit its text; long bar tooltips wrap;
  menus widen to fit their longest label (up to 560 px) instead of cutting labels off;
  no "Font \"\" is not available" line in the log at start.
- Fixes from testing on Windows: floating windows use Omarchy's sizes in logical
  pixels, so they are 64% wide at 125% scaling, not 57%; the Terminal scheme takes
  black, white, bright black and bright white from the theme as Omarchy does;
  running apps keep their place in the dock instead of reordering on every focus
  change; with an auto-hiding taskbar the dock sits above it; shorter dock option
  labels; message boxes always open in front (one behind other windows froze the
  Admin edition); a failed bar TRAY button shows an on-screen message instead of a
  message box. The README says that the Admin edition does not tile windows running
  as administrator.
- **Super + `** is Omarchy's drop-down console: the first press starts your terminal
  and drops its window over the top half of the screen, on top; the key then hides
  and shows it. It floats outside the tiles, and pause, exit and crash recovery take
  it off the top. It takes Windows Terminal's own Win + ` quake key.
- The scratchpad holds up to eight windows per workspace, like Omarchy's special
  workspace: **Super + Alt + S** adds the focused window or takes it out again, and
  **Super + S** shows them all or hides them all.
- **Status indicators** on the bar, as Omarchy's: Stay awake, pending reminders,
  microphone off and (with more than one keyboard layout) the current layout. The
  icon shows the most urgent one and the tooltip lists them all; nothing shows when
  there is nothing to say. New bars have it at the right; add it to an existing bar
  with Appearance > Top bar > Bar items.
- Reminders survive a restart or sign-out, as Omarchy's do: they are kept in
  `reminders.json` and come late if the PC was off. `remind 25 Stretch` adds a
  message to show instead of "25 minutes are up."
- **Screensaver and idle lock** (Appearance > Screensaver and idle lock), Omarchy's
  idle settings: a themed screensaver with a large clock, and locking the PC, each
  after a chosen time without input. Both are off by default, since Windows has its
  own lock timeout. Neither starts while a video or another app keeps the screen on,
  or while Stay awake is on.
- **Super + Print** picks a color from the screen, as Omarchy's color picker does:
  the screen freezes under a crosshair with the color and its hex value beside the
  pointer, and a click copies the hex value. Escape or a right-click cancels. It takes
  Windows' Super + Print screenshot key; remove the line from binds.conf to keep it.
- The on-screen message is an Omarchy-style card in the theme's colors and font: an
  icon, the text and, for volume and brightness, a level bar. Volume keys, mute and
  the brightness keys now show it too.
- The selection, selected-text and link colors of classic Windows controls (list
  boxes, edit boxes, menus and older dialogs) follow the theme accent, as Omarchy's
  GTK and Qt apps do. Window and text colors stay as they are. The originals are
  journaled and put back on exit, Restore and after a crash.
- **Font > Install JetBrains Mono for Windows Terminal and other apps** installs the
  bundled JetBrains Mono Nerd Fonts for this Windows user (no administrator rights),
  so the themed Terminal profile uses Omarchy's font. Like every other change it is
  undone on exit and after a crash, and put back at the next start while turned on.
  Fonts you already had are never touched.
- **One font**, as omarchy-font-set: choosing a font on the Font page sets it for the
  menus, the top bar and the themed Windows Terminal profile. Terminal uses it when
  the font is installed in Windows (it cannot see the fonts bundled beside the exe).
- The themed Windows Terminal profile and VS Code theme come back at every start
  once turned on. Exit still takes them away, like every other change to Windows;
  before, they stayed away until added again. **Restore the original Windows
  appearance** turns them off.
- The theme list shows a colour swatch for each theme (background, accent, text and
  red), as Omarchy's theme switcher shows a preview.
- The theme also sets the Windows accent palette, the lighter and darker shades of
  the accent that Start and the taskbar use, so they match the theme instead of
  Windows' last accent. It has its own journal and is restored with the accent.
- **Window transparency** (Appearance > Window transparency): Omarchy's opaque,
  transparent and glassy modes in one choice. Each sets the active and inactive
  window opacity and the app window material together.
- **VS Code colors** (Appearance > VS Code colors), as Omarchy themes VS Code: the
  theme's colors fill Omarchy's VS Code color theme, installed as a local "Omarchy
  Overlay" theme for VS Code, VS Code Insiders, VSCodium and Cursor, and selected in
  settings.json. Only `workbench.colorTheme` changes there; comments and every other
  setting stay. It follows theme changes, and Put back, exit and a crash restore the
  theme you had unless you picked another one since.
- **Dock**, after Omarchy's: on each monitor whose workspace uses the floating
  layout, it shows the pinned apps (the app picker's favorites) and then the other
  running apps, with a running dot. Click brings the app forward and cycles through
  its windows (this workspace, then minimized, then other workspaces) or starts it;
  middle-click opens a new window; right-click offers New window, Keep in dock /
  Remove from dock and Close. The **Windows taskbar** page sets it to show on
  floating workspaces (default), always (auto-hiding at the bottom edge) or never.
  An app whose window flashes for attention gets an accent ring until you bring it
  forward, as its taskbar button would flash.
- **Floating layout**, Omarchy's floating workspace mode: a workspace set to it stops
  tiling. Each window is placed once at min(1100, 64%) × min(760, 68%) of its
  monitor, centred and stepped 24 px in a cycle of five, then left where you put it;
  new windows open the same way. Switching back to a tiling layout re-tiles them.
  Windows floated with **Super + T** or an app rule stay floating. Exit, a crash and
  `RECOVER.cmd` put the windows back as for the tiled layouts.
- **Super + Shift + L** and **Super + Alt + L** change the layout of the workspace in
  front only. Before, they changed the default for every workspace unless that
  workspace already had a layout of its own.
- The Windows Terminal scheme uses each theme's own 16 terminal colors, cursor and
  selection from its `colors.toml`. Before, every theme but Neon Ridge got Tokyo
  Night's terminal colors.
- **Super + Shift + Backspace** turns the gaps off or back on, as in Omarchy, instead
  of opening the gaps page.
- The Admin zip carries only the stock Omarchy theme backgrounds, not ones from
  `~/.config/omarchy/themes`.
- Release builds run on GitHub Actions: a `v<version>` tag builds both zips and
  makes a draft release from them.
- More of Omarchy's keys are in the default binds.conf: **Super + Minus / Equal**
  resize the focused tile (Shift for height, Alt for a little, Ctrl for a lot),
  **Super + Alt + L** cycles the layout back, **Super + Ctrl + Z** and
  **Super + Ctrl + Alt + Z** zoom in and reset with Windows Magnifier,
  **Super + Ctrl + D** opens Display settings, and **Super + Ctrl + Alt + T / B**
  show the time and battery.
- **Super + F** is real full screen now: the window covers its whole monitor,
  above the bar, until you press it again. **Super + Alt + F** still maximizes.
- **Super + Alt + Home** saves the focused window's width and **Super + Home**
  restores it.
- **Ctrl + Alt + Tab** and **Ctrl + Alt + Shift + Tab** focus the next and previous
  monitor, and **Super + Shift + Alt + arrows** move the focused monitor's windows
  to the monitor in that direction.
- New commands: `resize width|height <pixels>`, `cycle-layout-back`, `mic-mute`,
  `zoom-in`, `zoom-reset`, `full-screen`, `save-width`, `restore-width`,
  `focus-next-monitor`, `focus-previous-monitor` and `move-workspace-left`,
  `-right`, `-up` and `-down`.
- **Window groups**, as on Omarchy: **Super + G** groups the focused window,
  **Super + Alt + arrows** move a window into the group in that direction,
  **Super + Alt + Tab** switches inside the group and **Super + Alt + G** takes a
  window out. A group shares one tile and shows one window at a time.
- **Reminders**: **Super + Ctrl + R** picks a time and a Windows notification
  shows when it is up; **Super + Ctrl + Alt + R** lists them and
  **Super + Ctrl + Shift + R** clears them.
- **Super + Alt + Print** starts a Snipping Tool screen recording and
  **Super + Ctrl + Print** copies text from the screen.
- New commands: `toggle-group`, `leave-group`, `move-into-group-left`, `-right`,
  `-up`, `-down`, `next-in-group`, `previous-in-group`, `remind <minutes>`,
  `show-reminders`, `clear-reminders`, `record-screen` and `extract-text`.

## 0.5.0

- **Super + left drag** moves a window and **Super + right drag** resizes it, as on
  Omarchy. Dropping a tiled window on another tile swaps them, and resizing a tile
  moves its neighbours with it.
- **Super + mouse wheel** switches to the previous or next workspace, as on
  Omarchy. A touchpad's small scroll steps add up to one workspace per notch.
- New commands for binds.conf: `volume-up`, `volume-down`, `mute`,
  `brightness-up` and `brightness-down` (external monitors over DDC/CI),
  `toggle-gaps`, `show-time` and `show-battery`.
- **Layout > Remember layout, main share and gap for each set of monitors** switches
  them when you dock, undock or plug in a screen.
- **Background > Apply to** sets a background on one monitor only. The originals
  are recorded and put back on exit.
- **Appearance > Window corners**: square, round or slightly round.
- **Appearance > Windows behavior** switches the taskbar alignment, Task View
  button and search box, file extensions and hidden files in File Explorer, Snap
  and its parts, focus follows mouse and window animations. Every original is
  recorded and restored.
- Right-click an app on the bar to close it.
- The key hook re-registers every 15 seconds, so a program started later cannot
  take Omarchy Overlay's keys first.
- The README now describes running the Standard edition from a flash drive.
- In a winget install, **Make this folder portable** now warns that the next
  `winget upgrade` deletes the portable profile, and saves only if you confirm.

## 0.4.1

- Omarchy Overlay is now licensed under the GPL-3.0 (GPL-3.0-only). Releases up to
  0.4.0 were MIT.
- Install with Scoop or winget; see Getting started. Each release now has a
  `SHA256SUMS.txt` with the checksum of every zip.
- When installed by winget, config files are kept in
  `%LOCALAPPDATA%\OmarchyOverlay\config`, because winget replaces the program's
  folder on every upgrade.

## 0.4.0

- Two new layouts. Centered main puts the main window in the middle and shares
  the rest between a right and a left column, for wide monitors. Monocle makes
  every window fill the monitor: focus up and down steps through them, left
  and right moves to the next monitor.
- Menus follow Omarchy's menu card: square corners, a 2 px border, a faded
  search line, the accent color on the selected row and Omarchy's sizes.
- The app picker is a grid like Omarchy's launcher: large app icons with their
  names, 7 across and 4 rows at a time, moved with the arrow keys.
- The README shows screenshots of the menu, the app grid and the Tokyo Night look.
- Workspaces are now Omarchy Overlay's own, ten of them, no longer Windows virtual
  desktops. **Super+1** to **Super+0** show a workspace, **Super+Shift+number** moves
  the focused window there and follows it, and **Super+Shift+Alt+number** moves it
  and stays. Windows of the other workspaces wait off-screen; choosing one with
  Alt+Tab or the taskbar shows its workspace. Exit, pause and crash recovery put
  every window back.
- **Super+O** pops a window out: it floats at 1300x900 in the middle of the screen,
  on top of the tiles and on every workspace. Press it again to put the window back.
- Startup retries hotkey registration once after a second when none registered,
  which happens when a copy that was just closed still holds every key, and logs
  the Windows error code for each key that failed.

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

- Every Omarchy theme is built in, replacing the old
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
