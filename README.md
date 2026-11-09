# Omarchy Overlay

A keyboard-first tiling window manager for Windows 11, inspired by the
[Omarchy](https://omarchy.org) desktop. It's written in Rust.

> **Not affiliated with Omarchy or its authors.** It's an independent project
> that brings an Omarchy-style workflow to Windows.

**Status: early.** This is version 0.1.0. It comes in two editions built from the
same exe: **Standard** runs from its own folder with no installation and no
administrator rights; **Admin** installs itself and runs elevated.
Pick Standard on a PC where you are not an administrator, or to try it out. Pick
Admin on your own PC when its keys must also work in apps running as administrator.

## What it does

- **Tiles windows:** main-stack, columns, rows, grid, dwindle and a scrolling
  layout inspired by Niri. You can resize windows by dragging their shared edges.
- **Super keys:** an app picker, a command menu, a keybind list and window
  actions, defined in `config\binds.conf`.
- **Top bar:** Windows virtual desktops as workspaces, the active app, a clock,
  a tray and modules.
- **Themes:** every Omarchy theme (Neon Ridge, Tokyo Night, Catppuccin,
  Gruvbox, Nord and more) built in, plus Mica and
  Acrylic materials and window opacity. It can apply the theme to Windows'
  own settings and puts the originals back on exit.
- **Recovery:** if the program exits or crashes, it restores the taskbar,
  window positions and transparency. `RECOVER.cmd` does the same by hand.

## Getting started

1. Build it (see below), or download a release when one is published.
2. Put `OmarchyOverlay.exe` in a folder with `assets\fonts` (as `fonts`) and `config\examples`
   folders from this repository.
3. Run `OmarchyOverlay.exe`. The menu opens at start.
4. Press **Super + Space** for the app picker, **Super + K** for every keybind,
   **Super + Alt + Space** for the menu and **Super + Escape** for the System menu.
5. To give every key back to Windows, choose **Trigger > Toggle > Pause Omarchy Overlay keys**.

Its logs and state are kept in `%LOCALAPPDATA%\OmarchyOverlay`.

### Admin edition

The Admin release has an `edition-admin` file beside the exe, which adds
**Install (admin)…** to the tray menu. Installing asks for administrator rights once, then:

- copies the exe, `fonts` and `RECOVER.cmd` to `%ProgramFiles%\OmarchyOverlay`;
- adds a logon task, **Omarchy Overlay**, that starts it elevated for the user who
  installed it, so its keys also work in apps running as administrator;
- adds a Start menu shortcut that runs that task, and an entry in **Settings > Apps**.

Apps you start from it still open without administrator rights.
**Uninstall…** in the tray (or Settings > Apps) restores the desktop and removes all
of it. Settings in `%LOCALAPPDATA%\OmarchyOverlay` are kept unless you run
`OmarchyOverlay.exe --uninstall --remove-settings`. From a prompt:
`OmarchyOverlay.exe --install` or `OmarchyOverlay.exe --uninstall`.

### Keys

- **Setup > Keymap** chooses **Super keys and Ctrl+Alt keys** (default),
  **Super keys only (Omarchy laptop)** or **Ctrl+Alt keys only**.
- Super keys come from `config\binds.conf`. Omarchy Overlay refuses Super + L,
  Ctrl + Alt + Delete and Ctrl-only keys. Plain Alt keys work, with a warning in
  the log, because they take the shortcut from the focused program.
- No app is special: every app, including CAD tools, follows its app rule.
- **Super + Alt + W** opens the Selected window panel for the window you were
  in: float or tile, opacity, maximize, main window, next monitor, scratchpad,
  the full app rule, always float and close. **Super + Backspace** toggles that
  window to 90% opacity.
- **Super + Ctrl + D** stays with Windows (New desktop), because new Omarchy Overlay
  workspaces are Windows desktops.
- After an Omarchy Overlay Super key, releasing Super does not open Start.
- In the Standard edition, windows running as administrator do not pass keys to
  Omarchy Overlay; click an ordinary window first. The Admin edition has no such limit.
- **Setup > Keyboard conflicts** lists keys another program already holds, with
  free alternatives.

### Look

Out of the box Omarchy Overlay looks like Omarchy on Hyprland:

- A Waybar-style top bar: the menu and workspaces on the left, the clock in the
  middle, tray and status on the right, in JetBrainsMono Nerd Font.
- Square window corners with a 2 px border in the theme's active color on the
  focused window and its inactive color on the rest, with gaps between tiles.
- Walker-style launcher and menus: centred, square, 2 px accent border, an 84%
  opaque background and the selected row filled with the theme's border color.
- The theme sets the Windows accent color and its first background, at start
  and whenever you choose one. **Super+Ctrl+Space** moves to the next background in the theme's
  `backgrounds` folder.

Every Windows setting this changes is recorded first and restored on exit, from
the recovery menu, and after a crash.

### Apply theme to Windows

Choose **Appearance > Apply theme to Windows**. Each line shows the current
value; **Enter** changes it.

- Windows dark/light mode, Transparency effects, Accent color on Start and taskbar.
- **Accent color** shows the theme's accent (Neon Ridge: `#2A3A78`). The theme
  sets it as the Windows accent too; the original returns on exit.
- **Other apps' window frames follow the theme** gives other programs a dark
  frame with the theme's border and caption colors. **Mica or Acrylic** adds a
  material; if Windows transparency is off, Omarchy Overlay says blur will look solid.
- **Background** and **Windows Terminal colors** are on the same page.
- **Restore the original Windows appearance** puts every original value back.
  Omarchy Overlay records the originals first and also restores them on exit and after
  a crash.
- **Apps Omarchy Overlay cannot theme** lists Office, Teams, Chrome/Edge and CAD programs
  with where each one's own theme setting is.

### Windows that always float

**Setup > Tiling > Always-float list** shows the built-in entries (Run, Open
With, Properties, file copy/move/delete progress, replace and confirm prompts)
and your own. **Add an open window** picks one from the windows that are open.
Switching a built-in entry off asks twice. These windows are never tiled and
stay where Windows opens them.

### Web apps

**Super + A** brings ChatGPT forward and **Super + Shift + A** Claude. If no
window is open, Omarchy Overlay starts the app from Chrome's **Chrome Apps** Start-menu
folder; if that shortcut is missing it offers to open the site. To make the app:
open the site in Chrome, then **⋮ > Cast, save and share > Install page as app**.

## Config files

Omarchy Overlay reads these files from the `config` folder beside `MBOmarchy Overlay.exe`.
Every file is optional. Omarchy Overlay writes only `config\examples` (refreshed at each
start) and never changes your copies. To change a file, copy it from
`config\examples` into `config`, edit it in Notepad, then choose **Update >
Config > Reload config files**. If a file has a mistake, Omarchy Overlay keeps the
built-in version; the first line of the menu then says how many files have
problems, and **Update > Config > Config file problems** lists each file and line.

| File | Controls |
| --- | --- |
| `config\binds.conf` | Super keybinds |
| `config\menu.json` | The menu: lists, labels, icons, order and commands |
| `config\webapps.json` | Web apps for the `webapp` command |
| `config\always-float.json` | Built-in always-float entries switched off, and your own windows |
| `config\themes\` | Extra or replacement themes, as `<name>.toml` files or Omarchy theme folders |

### binds.conf

One key per line: key, description, command. Lines starting with `#` are notes.
Keys are written `Super+Shift+A`; modifiers are Super, Ctrl, Alt and Shift.

```text
Super+Return, Terminal, app terminal
Super+Shift+E, Email, app email
Super+A, ChatGPT, webapp ChatGPT
Super+1, Switch to workspace 1, workspace 1
```

### menu.json

A list of menus. Each has an `id`, a `title` and `items`; each item has a
`label`, an `icon` (a Nerd Font code in hex, optional), a `command` and optional
`"confirm": true` (Enter twice). The menu with id `main` is the top level;
`menu <id>` opens another one.

```json
{
  "menus": [
    {
      "id": "system",
      "title": "System",
      "items": [
        { "label": "Lock", "icon": "F023", "command": "lock" },
        { "label": "Shut down", "icon": "F011", "command": "shutdown", "confirm": true }
      ]
    }
  ]
}
```

### webapps.json

```json
{
  "apps": [
    { "name": "ChatGPT", "url": "https://chatgpt.com", "shortcut": "ChatGPT", "window_title": "ChatGPT" }
  ]
}
```

`shortcut` is the name in the Start menu's Chrome Apps folder; `window_title` is
text in the app window's title.

### always-float.json

```json
{
  "disabled": [],
  "windows": [
    { "executable": "calc.exe", "window_class": "", "title_contains": "" }
  ]
}
```

`disabled` takes `run`, `open-with`, `properties`, `file-operations` and
`confirm-prompts`. Each window needs a program, a window class or title text.

### themes

The built-in themes are the Omarchy themes, embedded from their `colors.toml`
files. Add or replace a theme in either form:

- `config\themes\<name>.toml`: an Omarchy `colors.toml` file; the file name
  is the theme name.
- `config\themes\<name>\colors.toml`: an Omarchy theme folder, copied as it
  is from `~/.config/omarchy/themes/<name>` on an Omarchy machine. A
  `backgrounds` folder beside it is kept with the theme.

A theme with the same name as a built-in one replaces it. Keys Omarchy adds
later are ignored, so newer Omarchy themes still load.

```toml
accent = "#7daea3"
background = "#282828"
foreground = "#d4be98"
mode = "dark"
```

`scripts/import_omarchy_themes.py` refreshes the embedded themes from an
Omarchy install; `--out DIR` also writes theme folders to copy to a PC.

### Commands

Used in binds.conf and menu.json.

| Command | Does |
| --- | --- |
| `apps` | Open the app picker |
| `menu` | Open the Omarchy Overlay menu |
| `keybinds` | Show all keybinds |
| `open-windows` | Search open windows |
| `window-panel` | Compact panel for the selected window |
| `reload-config` | Reload config files |
| `restart` | Restart Omarchy Overlay |
| `exit` | Restore the desktop and exit Omarchy Overlay |
| `pause-keys` | Give all keys back to Windows until resumed |
| `toggle-bar` | Show or hide the top bar |
| `keep-awake` | Keep the screen awake |
| `lock` | Lock the PC |
| `sleep` | Sleep |
| `sign-out` | Sign out of Windows |
| `restart-pc` | Restart the PC |
| `shutdown` | Shut down the PC |
| `task-view` | Windows Task View |
| `notifications` | Windows notifications |
| `quick-settings` | Windows Quick Settings |
| `clipboard` | Windows clipboard history |
| `emoji` | Windows emoji panel |
| `capture` | Screenshot or screen recording (Snipping Tool) |
| `next-desktop` | Next workspace |
| `previous-desktop` | Previous workspace |
| `move-to-desktop` | Move a window to another workspace (Task View) |
| `apply-windows-theme` | Apply the theme to Windows |
| `restore-windows-appearance` | Restore the original Windows appearance |
| `update-profile` | Update the active profile from current settings |
| `guide` | Open the Omarchy Overlay guide |
| `log` | Open the Omarchy Overlay log |
| `config-folder` | Open the config folder |
| `releases-folder` | Show where Omarchy Overlay releases are kept |
| `restore-desktop` | Restore every window to its original place |
| `menu <id>` | Opens that list of menu.json, for example `menu system` |
| `page <id>` | Opens a settings page: `theme`, `font`, `background`, `gaps`, `main-size`, `layout`, `columns`, `opacity`, `inactive-opacity`, `materials`, `animations`, `focus-outline`, `osd`, `app-rules`, `default-rule`, `add-rule`, `always-float`, `always-float-add`, `bar`, `bar-items`, `clock`, `bar-height`, `bar-opacity`, `workspace-layouts`, `keyboard`, `conflicts`, `keymap`, `tap-super`, `taskbar`, `profiles`, `windows-theme`, `windows-checklist`, `webapps`, `install-webapp`, `remove-apps`, `custom-apps`, `version`, `terminal-colors`, `config-errors` |
| `app <role>` | Starts or shows an app: `terminal`, `browser`, `browser-private`, `files` (the file manager Windows opens folders with), `explorer` (File Explorer itself, at This PC), `email`, `new-email`, `calendar`, `teams` |
| `webapp <name>` | Brings a web app from webapps.json forward |
| `workspace <1-9>` | Switches Windows desktop |
| `move-to-workspace <1-9>` | Opens Task View to move the window |
| `layout <name>` | `main-stack`, `columns`, `rows`, `grid`, `dwindle`, `scrolling` |
| `theme <name>` | Switches theme, for example `theme neon-ridge` |
| `settings <page>` | Opens a Windows Settings page, for example `settings sound` |
| `action <id>` | Runs a window action, for example `action close-window` |
| `launch <file> [arguments]` | Runs a program or script stored inside the config folder, as you |


## Build and check

On Windows, with Rust (rustup.rs) and Visual Studio 2022 Build Tools
("Desktop development with C++"):

```powershell
rustup target add x86_64-pc-windows-msvc
cargo test
cargo build --release --target x86_64-pc-windows-msvc
```

The program is built to `target\x86_64-pc-windows-msvc\release\OmarchyOverlay.exe`.

On Linux, with cargo-xwin:

```sh
cargo test
cargo clippy --target x86_64-pc-windows-msvc -- -D warnings
cargo xwin build --release --target x86_64-pc-windows-msvc
```

The `tests\windows-*.ps1` scripts are integration tests for a real Windows
desktop. They open disposable windows and turn tiling on, so run them on a test
machine or VM, never on a desktop you're using. `scripts/test_windows.py` runs
them in a libvirt Windows guest through `scripts/vm.py`. Set `OVERLAY_VM`,
`OVERLAY_VM_USER` and `OVERLAY_VM_GUEST` to match your VM.

### Releases

After the release build, `python3 scripts/package.py` writes two zips to `dist\`
from the same exe:

- `OmarchyOverlay-<version>-standard.zip`: the exe, `RECOVER.cmd`, `fonts\` and
  `config\examples\`. Unzip and run; no administrator rights.
- `OmarchyOverlay-<version>-admin.zip`: the same, plus `INSTALL.cmd` and the
  `edition-admin` marker. Run `INSTALL.cmd` to install (see Admin edition).

`python3 scripts/package.py --selftest` checks both file lists and that the two exes match.
Two VM suites test the zips and are not in the default list: `standard` runs the
Standard release with a limited token, and `install` installs the Admin release,
checks it, and uninstalls it (it runs elevated, so the VM user must be an administrator).

## License

MIT. See [LICENSE](LICENSE).
