# Test queue

## Build 11 — commit 0a27206 — building (run number added when done)

Adds: single-window square (K2), pseudo-tile (K3), typed commands in the menu (O6),
theme install from git (TH1).

## Build 10 — commit ddcd9bd — Actions run 45652725256 — test this one now (has all of 9)

Adds: backgrounds in the Standard zip (BG1), full-screen desktop, group window keys,
close-all, hibernate, voice typing (K1).

## Build 9 — commit 4c2a8d5 — Actions run 45644597372

Retest: B1 (icon), D8 with the taskbar auto-hiding, TRAY card and tooltip, menu
labels, and the log at start (no `Font ""` line). Plus P1 and W1 from build 8.

## Build 8 — commit f72a38a — Actions run 45641570012

Adds: bar position (P1), per-monitor workspaces (W1, two monitors needed).

## Build 7 — commit 2c3f776 — Actions run 45638788963

Also has: Windows wallpaper cache copy (recheck R1/exit), gap toggle takes edge gaps
(recheck F7 and note what changes on screen), group tabs (G1), defaults merge (M1).

Adds: new default keys and menu entries merged into existing `config\binds.conf` and
`menu.json` (M1), group tabs (G1).

- **M1** Before starting build 7, note `config\binds.conf`. After the first start:
  new keys (Super+Print, Super+`) are at the end under "# Added by Omarchy Overlay";
  lines you never edited follow the new defaults; your own changes stay;
  `binds.conf.before-0.5.0` holds the old file. Same for menu.json (Appearance shows
  VS Code colors, Window transparency, Screensaver and idle lock).

Newest build first. Test IDs stay the same across builds; a retest names the ID again.
Standard zip unless marked Admin. Have 3–4 ordinary windows open (Notepad, File
Explorer, a browser) before starting.

## Build 6 — commit 02242fa — Actions run 45636336058

First build with the manifest fix, so the Actions exe starts. Retest **F1** (64% wide
at 125%), **T1** (black, bright black, bright white now from the theme), **D1** (icons
keep their order), **D8** with the taskbar on auto-hide (dock above it), and the
Admin TRAY button (an on-screen message, no frozen overlay).

Adds to build 5: bar status indicators (B1), multi-window scratchpad (S1),
drop-down console (C1).

## Build 5 — commit 7fd09c4 — Actions run 45629843370

Everything below, plus the per-user font install, classic-control accent colors,
OSD card, color picker, idle screensaver and lock, and persistent reminders.

## Tests

### Floating layout and layout keys
- **F1** Super+Shift+L until the card says "This workspace: Floating". Windows jump to
  the middle, about two-thirds of the screen, staggered 24 px apart.
- **F2** Drag a window by its title bar and with Super+drag: it stays where dropped.
- **F3** A new Notepad opens floating in the middle, not tiled.
- **F4** Super+2 tiles normally; Super+1 is still floating, windows where they were.
- **F5** Cycling back to a tiling layout tiles every window again. A window floated
  with Super+T before entering floating stays floating.
- **F6** Super+Alt+L cycles backwards, this workspace only (workspace 2 unchanged).
- **F7** Super+Shift+Backspace turns gaps off and back on.
- **F8** Exit (tray → Exit): every window returns to where it was before start.

### Dock (on a floating workspace)
- **D1** Dock at the bottom centre with running apps; dot under each, wider accent
  dot under the app in front.
- **D2** Click brings the app forward; with two Notepads, clicking again switches to
  the other.
- **D3** A minimized window comes back when its app is clicked.
- **D4** Middle-click opens a new window of the app.
- **D5** Right-click → Keep in dock: icon stays after closing all its windows (no
  dot); clicking it starts the app. Right-click → Remove from dock: icon goes.
- **D6** Right-click → Close window closes it.
- **D7** Switching the workspace to a tiling layout hides the dock.
- **D8** Menu → Appearance → Windows taskbar: "Always" hides the dock until the
  pointer touches the bottom edge; "No dock" removes it; set back to floating.
- **D9** An app flashing for attention (a finished download in a background browser)
  gets an accent ring until clicked.
- **D10** A full-screen video hides the dock.

### Theme and Windows look
- **T1** Theme Gruvbox, then Appearance → Windows Terminal colors → Add. In Terminal's
  Omarchy Overlay profile, `Write-Host red -ForegroundColor Red` is Gruvbox red
  (#ea6962), not Tokyo Night pink.
- **T2** Appearance → Window transparency: Glassy (75% + Acrylic title bars),
  Transparent (barely), Opaque (solid). The current mode is ticked.
- **T3** With a strong accent theme and Settings' "Show accent color on Start and
  taskbar" on, Start and the taskbar use the theme accent shades. Exit restores them.
- **T4** Appearance → VS Code colors → Theme VS Code: VS Code switches to "Omarchy
  Overlay"; other settings.json lines and comments untouched; a theme switch follows;
  Put back restores the old theme. SKIP without VS Code/Insiders/VSCodium/Cursor.
- **T5** Appearance → Theme shows a four-colour swatch beside each theme.
- **T6** Appearance → Font → Cascadia Mono: menus and the top bar switch to it.
- **T7** Exit and start again: Terminal and VS Code colours come back by themselves.
- **T8** Appearance → Font → Install JetBrains Mono: Terminal's Omarchy Overlay profile
  uses JetBrains Mono after restarting Terminal. Exit removes the fonts again.
- **T9** Selected text in Notepad and in an older dialog's list highlights in the theme
  accent. Exit restores Windows' blue.

### Shell
- **O1** Volume keys and mute show a card with an icon, "Volume N%" and an accent bar.
- **O2** Brightness keys show the same with a brightness icon (DDC monitors only).
- **O3** Super+Print freezes the screen with a crosshair and a colour label; a click
  copies "#rrggbb" and shows "#rrggbb copied"; Esc and right-click cancel.
- **O4** Appearance → Screensaver and idle lock → Screensaver 2:30: after 2.5 minutes
  idle a themed clock screen appears; moving the mouse closes it. Not during a
  full-screen video. Set back to Off.
- **O5** Lock after 5 minutes: the PC locks after 5 minutes idle. Set back to Off.
- **O6** (build 11 or later) Menu (Super+Space), type `remind 2 Test message`, Enter.
  Exit and restart within 2 minutes: the notification still comes with "Test
  message". Earlier builds had no way to type a command; skip O6 there.

### Build 6 only
- **B1** Appearance → Top bar → Bar items: add Status indicators. Toggle Stay awake
  (menu) and mute the mic: the bar shows the icon; tooltip lists both. With two
  keyboard layouts the layout code shows.
- **S1** Super+Alt+S on two windows ("In the scratchpad"); Super+S hides both, Super+S
  again shows both; Super+Alt+S on one again takes it out.

- **C1** Super+` starts the terminal over the top half of the screen, on top; Super+`
  hides it and shows it again; it stays out of the tiles. Exit: it is no longer
  always on top.

- **G1** Super+G on a tiled window, then Super+Alt+arrow to move a neighbour into
  the group: a tab row appears at the top of the tile, the shown window's tab in the
  accent colour; clicking the other tab shows that window; Super+G again ungroups and
  the tile is full height again.

- **P1** (build 8) Appearance > Top bar > Bar position > bottom: the bar moves to the
  bottom of every monitor, tiles end above it, the dock sits above it; back to top.

- **W1** (build 8, two monitors) Layout > "Each monitor has its own workspaces" on.
  With windows on both monitors, focus monitor A and press Super+2: only A's windows
  go; B is unchanged. Click a window on B: the bar shows B's workspace (1). Super+1 on
  A brings A's windows back. Super+Shift+3 sends a window to workspace 3 of its own
  monitor. Turning the setting off brings every hidden window back. Exit restores all.

- **K1** (build 10) Super+Ctrl+Alt+F: bar and gaps go; again: both back. In a group,
  Super+Alt+2 shows its second window. Trigger > Voice typing opens Windows
  dictation. System > Hibernate asks first (skip actually hibernating a VM if it can't).
  `close-all` from the command line closes the workspace's windows, each app asking
  about unsaved work. Standard zip has `themes\<id>\backgrounds` (BG1).

- **K2** (build 11) One tiled window on a workspace, Super+Ctrl+Backspace: it becomes
  a centred square ("A lone window stays square"); a second window tiles normally;
  closing it brings the square back; the key again fills the screen.

- **K3** (build 11) Super+P on a tiled Notepad: it returns to its own size, centred in
  its tile; Super+P again fills the tile. Exit restores it as usual.

- **TH1** (build 11) With Git for Windows installed: menu, type
  `theme-install bjarneo/omarchy-ash-theme`, Enter. "Installing…", then the theme
  switches; `config\themes\ash` holds only colors.toml and backgrounds\. Without git:
  a message saying to install it.

### Always
- **R1** After any crash or kill (Task Manager → End task), start again or run
  RECOVER.cmd: windows, taskbar, accent, wallpaper, Terminal/VS Code colours, fonts
  and selection colours are all back to normal.
