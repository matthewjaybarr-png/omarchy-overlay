# Full Windows theming — research and plan

Date: 2026-11-24. Status: direction agreed 2026-11-24; no code yet.

## Decisions (2026-11-24)

1. **Omarchy is the design source for all theming.** Every look, color and behavior
   follows Omarchy and its themes. New features we add keep to the same style. We
   don't ship older-Windows looks (7, XP, classic) as built-ins.
2. **Three layers:** the Standard edition, the Admin edition, and optional add-ons
   (the third-party engines in Tier E). People may do what they want to their own PC:
   add-ons are offered, installed only with consent, and always come with a clear
   warning about their risks.
3. **Order of work:**
   - **Phase 1:** make the Standard edition look, feel and behave as much like
     Omarchy as Windows physically allows.
   - **Phase 2:** do the same for the Admin edition, using what elevation adds.
   - **Phase 3:** start full Windows theming with add-ons, only once Standard and
     Admin are fully ironed out.

The gap audit is done: `2026-11-24-omarchy-gap-audit.md` holds the Phase 1 and
Phase 2 task lists in build order.

### Phase 1 items named so far

- **Floating windows mode.** Today single windows float (Win+T, app rules, the
  always-float list). Add the Omarchy-style mode in which a whole workspace stops
  tiling: windows keep where the user put them, Super + drag moves and resizes them,
  and switching back re-tiles. The gap audit confirms Omarchy's exact keys and
  behavior. It must work the same way per workspace and survive restart, crash
  recovery and `RECOVER.cmd` like the tiled layouts do.
- **A dock, and the taskbar fully out of the way.** The taskbar can already be
  visible, auto-hidden or hidden (`bar.rs`, `taskbar_visibility.rs`, journaled and
  restored). Add an Omarchy-styled dock window: pinned and running apps in theme
  colors and font, launching through the existing unelevated launch path,
  optionally auto-hiding. With the bar plus the dock covering what the taskbar does,
  "hidden" becomes the default. Things the taskbar does that the replacement must
  still reach: the tray (the bar already has it), notifications and Quick Settings
  (Win+N / Win+A), Start (Win), and app flashing for attention.

The tier research below still stands. Phases 1 and 2 use Tiers S and A only.
Phase 3 uses Tier E.

## Goal

Theme as much of Windows 11 as third-party theming apps do (window chrome, controls,
taskbar, Start, cursors, sounds, lock screen, and whole "looks" such as an older
Windows), driven by the Omarchy theme the user picks. Put everything that is safe
and reversible into the **Standard** edition. Put what needs elevation into the
**Admin** edition. Offer anything that patches or injects into Windows only as an
optional, separately installed engine that the Admin edition detects and drives.

Hard rules carried over from the look-and-admin spec:

- Normal Windows behavior never breaks.
- Every change is journaled before it is made and restored by exit, `--recover`,
  `RECOVER.cmd` and uninstall.
- The Standard edition never needs administrator rights.

## The signing rule

Omarchy Overlay is heading for a code-signed release. A signed exe builds reputation
with SmartScreen, Smart App Control (SAC) and Defender, and that reputation belongs to
the signature, so one flagged file costs every release. Therefore:

- **`OmarchyOverlay.exe` never injects code, hooks another process or patches a
  system file.** It only calls documented or long-stable Win32/WinRT/registry
  interfaces, as it does today.
- Invasive work is done by third-party engines the user installs (from winget where
  possible) and can remove on their own. The overlay detects them, writes their
  settings, and stops cleanly if they are missing, blocked or broken.
- We ship no Microsoft-derived files: no `.msstyles` built from `aero.msstyles`, no
  Windows icons or sounds. Looks that need them link to their source; we don't bundle them.

## What Windows offers, by tier

Findings from the 2026-11-24 research pass (sources in the appendix).

### Tier S — Standard edition (per user, no injection)

| Area | Mechanism | Already in the overlay? |
|---|---|---|
| Dark/light, transparency, accent on Start/taskbar | HKCU `Personalize` | Yes (`windows_personalization.rs`) |
| Accent color | HKCU `DWM`/`Accent` DWORDs | Yes, without `AccentPalette` (`windows_accent.rs`) |
| Accent palette (Start/taskbar shades) | HKCU `Explorer\Accent\AccentPalette` (binary, 8 colors) | **No** |
| Wallpaper, per monitor | `IDesktopWallpaper` / `SPI_SETDESKWALLPAPER` | Yes |
| Window border, caption, caption text, backdrop, corners | `DwmSetWindowAttribute` on other apps' windows | Yes (`window_theme.rs`), unelevated windows only |
| Win32 system colors (classic dialogs, menus, highlight) | `SetSysColors` (current session only, so re-applied at logon) and HKCU `Control Panel\Colors` | **No** |
| Caption, menu and message fonts | `SPI_SETNONCLIENTMETRICS` | **No** |
| Cursor scheme | HKCU `Control Panel\Cursors` + `SPI_SETCURSORS` | **No** |
| Sound scheme | HKCU `AppEvents\Schemes` | **No** |
| Lock screen image | WinRT `UserProfilePersonalizationSettings` (still to check: does it work from an unpackaged exe?) | **No** |
| Windows Terminal | JSON fragment in `%LOCALAPPDATA%` | Yes |
| VS Code, other apps | `workbench.colorCustomizations`, etc. | **No** |
| A whole `.theme` file | Generated per user, stock visual style | **No** |

### Tier A — Admin edition (elevation, still no injection)

| Area | Mechanism |
|---|---|
| Frames of elevated windows | The elevated overlay can call `DwmSetWindowAttribute` on them. That a Standard instance can't is not yet checked (UIPI). |
| Sign-in / lock screen colors | HKLM `Themes\DefaultColors\Standard` (LogonUI reads it) |
| Lock screen image, machine wide | `PersonalizationCSP` registry values (supported editions to check) |
| System-wide theme files | Install `.theme` files into `%WINDIR%\Resources\Themes` so they appear in Settings |
| Font substitution | HKLM `FontSubstitutes` (no effect on XAML/Segoe UI Variable surfaces) |
| Terminal fragment for all users | `%ProgramData%` fragment |
| Installing and updating engines | winget, run from the elevated instance with consent |

### Tier E — Engines (Admin edition, optional, invasive)

| Engine | What it unlocks | How it works | License | Main risk |
|---|---|---|---|---|
| **SecureUxTheme** (namazso) | Custom `.msstyles`: every Win32 control, frame, menu, scrollbar (the "older Windows" look for classic apps) | Application Verifier DLL via HKLM IFEO on winlogon, explorer, LogonUI, SystemSettings; v4 applies themes from normal Settings; MSI; on winget | LGPL-2.1, signed | Login loop that needs WinRE to fix; Windows 11 support only stated up to 24H2 |
| **Windhawk** + styler mods | Taskbar, Start and notification center restyled through XAML overrides (Taskbar Styler, Start Menu Styler, Notification Center Styler) | Global injection, runs as a service; mods compiled locally | GPL-3.0 (stylers GPL-3.0) | AV/SAC blocks, breaks when Microsoft changes the shell XAML, no CLI to install mods |
| **ExplorerPatcher** | Windows 10 taskbar, older Start, classic context menus, File Explorer ribbon | `dxgi.dll` in `%WINDIR%`, Explorer restart | GPL-2.0 (only? to check) | Microsoft blocked it at 24H2 upgrade time; asks to turn SAC off |
| **Open-Shell** | Classic, XP and 7-style Start menus | Explorer hook (not covered by the research; to check) | MIT | Same class as ExplorerPatcher |
| StartAllBack, Start11, WindowBlinds | The same as above, commercial | Proprietary hooking | Proprietary | Paid; we can only detect them and point to them |

Things no option here changes without an engine: Settings, the new File Explorer
command bar and other WinUI/XAML apps keep Microsoft's look.

## Design

### 1. A "Windows look" as data

Today an Omarchy theme drives colors, accent, dark mode and wallpaper. Add a
**look** layer, separate from the color theme, so either one can change on its own:

```
config\looks\<id>\look.toml        # what to turn on, per tier
config\looks\<id>\cursors\          # optional .cur/.ani, user supplied or our own
config\looks\<id>\sounds\           # optional .wav
config\looks\<id>\style.msstyles    # optional, user supplied, needs SecureUxTheme
```

`look.toml` names the settings per area: syscolors (from the theme or fixed, e.g.
classic grey), fonts, corners (square for older looks), cursors, sounds, taskbar
(native / ExplorerPatcher Windows 10 / Windhawk style file), Start (native /
Open-Shell skin / Windhawk style file), visual style. Each area says which tier it
needs. The UI shows what's unavailable and why ("needs the Admin edition",
"needs SecureUxTheme"), but never hides it silently.

Built-in look: **Omarchy** only, colored from the current Omarchy theme. The look
layer exists so Phase 3 add-ons (a taskbar or Start style, a visual style) can be
added without changing how themes work. Users may make their own looks; we ship
no older-Windows looks and no `.msstyles`.

Pure functions turn (theme, look) into the values to write: syscolor table,
`AccentPalette` bytes, `.theme` text, Windhawk style JSON, Open-Shell settings.
These get unit tests, like `theme_files::accent_values` today.

### 2. One journal for the whole look

Each area becomes a module with the existing shape (`apply`, `pending`, `restore`,
first-original journal). A new `look_apply.rs` runs areas in order through the
`effects_gate`, so a theme switch and a look switch never interleave. Recovery,
`RECOVER.cmd` and uninstall restore all areas. Engine areas record their previous
engine settings and restore those. They never uninstall an engine the user had already.

### 3. Engines are adapters, not dependencies

`engines.rs` defines one adapter per engine: `detect()` (installed, version, running,
blocked?), `apply(settings)`, `restore()`. Rules:

- Detection is read only and runs in both editions, so the Standard UI can tell
  the user what the Admin edition would add.
- Installing an engine is an explicit button in the Admin edition. It shows a
  warning first (what it changes, what can go wrong, how to undo it), then runs
  `winget install` with the exact id. It never installs as part of picking a look.
- SecureUxTheme gets extra care: before any change it checks that WinRE is enabled
  (`reagentc /info`) and shows how to recover from a login loop. It's only offered
  on builds the engine states support for.
- If SAC is on, engine areas are disabled with an explanation rather than tried.
- An engine that fails or vanishes after a Windows update leaves its area at
  "native". The rest of the look still applies.

### 4. Edition split

| | Standard | Admin |
|---|---|---|
| Tier S areas | Yes | Yes |
| Tier A areas | Shown as "Admin edition" | Yes |
| Engine detection | Yes | Yes |
| Engine install and settings | No | Yes, opt-in per engine |
| Frames on elevated windows | No | Yes |

As today, the split follows `process_elevated()` and the install folder, not a build
flag, so one signed exe serves both zips.

### 5. Signing

- Candidates: **SignPath Foundation** (free code signing for open-source projects,
  built for GitHub Actions) and **Azure Artifact (Trusted) Signing** (low monthly
  cost; check if the user's country and account type qualify). Both avoid
  holding a private key on a PC. To decide with the user.
- Sign `OmarchyOverlay.exe` and the `INSTALL.cmd`-launched paths in `release.yml`.
  Add a check that the zip contains no unsigned PE files.
- The `--shell-open` helper is the same exe, so it's covered.
- Engines are signed (or not) by their own authors; we document which ones need
  SAC off.

## Stages

Stages 1–2 belong to Phase 1 (Standard). Stages 3–4 belong to Phase 2 (Admin).
Stages 5–6 are Phase 3 and start only after Phases 1 and 2 are done. Phase 1 begins
with an Omarchy gap audit: list every part of Omarchy's look, feel and behavior,
mark each as done, possible in Standard, Admin only, or not possible on Windows,
and turn the "possible" rows into tasks ahead of stage 1.

Each stage ends with: `cargo test`, clippy clean for `x86_64-pc-windows-msvc`, a VM
suite (`scripts/test_windows.py`) covering apply, restore and `--recover` after a
kill, a CHANGELOG entry, and README/Windows-behavior docs.

1. **Look model + Tier S colors.** `look.toml` parsing; `AccentPalette`; syscolors
   (with logon re-apply); nonclient fonts; the Omarchy and Classic looks without
   engines. VM test: values set, restored, survive a crash.
2. **Cursors, sounds, `.theme` export, lock screen.** Generate a per-user `.theme`
   that captures the current look ("Save as Windows theme"); lock screen via WinRT
   if it works unpackaged, otherwise via the `.theme`/Settings route.
3. **Admin Tier A.** Elevated-window frames (first confirm the UIPI behavior in the
   VM, Standard vs Admin); LogonUI colors; machine-wide theme folder; ProgramData
   Terminal fragment. VM test as an admin user with an elevated Notepad.
4. **Signing in CI.** SignPath or Trusted Signing in `release.yml`; SmartScreen
   and SAC check on a fresh VM with SAC on.
5. **Engine adapters, read only.** Detection for SecureUxTheme, Windhawk,
   ExplorerPatcher, Open-Shell, StartAllBack, Start11, WindowBlinds; shown in the UI.
6. **Engine adapters, write.** In order of value and safety: Windhawk styler settings
   (taskbar/Start colors from the theme), Open-Shell Start skins, ExplorerPatcher
   taskbar style, then SecureUxTheme visual styles last. Each adapter gets its own
   VM snapshot test: install engine, apply, Windows update not simulated, restore,
   uninstall engine, desktop normal.

Stages 1–4 are worth shipping even if stage 6 never happens.

## Open questions

1. ~~Signing~~ Decided 2026-11-24: **SignPath Foundation**, since the project is fully
   open source. Remaining question for when we apply:: SignPath Foundation (free, needs an OSS application and review) or
   Azure Trusted Signing (paid, needs identity validation)?
2. Which add-on first in Phase 3: Windhawk stylers (taskbar and Start in theme
   colors) or SecureUxTheme (window styles)?

## To verify before building

- That `DwmSetWindowAttribute` on an elevated window fails from an unelevated process.
- Whether `UserProfilePersonalizationSettings` works from an unpackaged exe.
- SecureUxTheme on 25H2/26H2, under HVCI and SAC.
- ExplorerPatcher's exact license (GPL-2.0-only vs or-later) before reading its code.
- Open-Shell's current Windows 11 status and mechanism.
- Whether Windhawk's mod settings can be written safely from outside (its format isn't documented).
- `PersonalizationCSP` edition support (Pro vs Enterprise/Education).

## Appendix — sources

- `.theme` format: https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/Controls/themesfileformat-overview.md
- DWM attributes: https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/dwmapi/ne-dwmapi-dwmwindowattribute.md
- SetSysColors: https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/winuser/nf-winuser-setsyscolors.md
- SystemParametersInfo: https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/winuser/nf-winuser-systemparametersinfow.md
- Lock screen API: https://github.com/MicrosoftDocs/winrt-api/blob/docs/windows.system.userprofile/userprofilepersonalizationsettings.md
- Terminal fragments: https://github.com/MicrosoftDocs/terminal/blob/main/TerminalDocs/json-fragment-extensions.md
- SecureUxTheme: https://github.com/namazso/SecureUxTheme (login-loop recovery in its wiki)
- msstyleEditor: https://github.com/nptr/msstyleEditor
- Windhawk: https://github.com/ramensoftware/windhawk, mods: https://github.com/ramensoftware/windhawk-mods
- ExplorerPatcher: https://github.com/valinet/ExplorerPatcher
- Mica For Everyone (DWM attributes from outside, no injection): https://github.com/MicaForEveryone/MicaForEveryone
- DWMBlurGlass (rejected: hooks dwm.exe): https://github.com/Maplespe/DWMBlurGlass
- 24H2 upgrade blocks on customization apps: https://www.tomshardware.com/software/operating-systems/microsoft-stifles-third-party-interface-customization-apps-in-windows-11-version-24h2
- 25H2 shares 24H2's code: https://techcommunity.microsoft.com/blog/windows-itpro-blog/an-it-pros-guide-to-windows-11-version-25h2/4457409
- Smart App Control and unsigned DLLs: https://support.microsoft.com/en-US/Windows/Security/Threat-Malware-Protection/smart-app-control-has-blocked-part-of-this-app
