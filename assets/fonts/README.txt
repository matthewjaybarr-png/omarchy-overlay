JetBrains Mono NL Nerd Font and JetBrainsMono Nerd Font 3.5.1 (Regular and Bold),
from the Nerd Fonts project (https://github.com/ryanoasis/nerd-fonts), licensed
under the SIL Open Font License 1.1 in OFL.txt.

  JetBrainsMonoNLNerdFont-*.ttf  menu text since 0.17 ("JetBrains Mono NL", no ligatures)
  JetBrainsMonoNerdFont-*.ttf    the menu icons, and the older "JetBrainsMono Nerd Font" choice

Orbit loads these files for its own windows only (AddFontResourceEx with
FR_PRIVATE). They are not installed, other programs cannot see them, and they
are released when it exits. MBOrbitSetup.exe copies this whole folder into
%LOCALAPPDATA%\OmarchyOverlay\fonts. Without the folder, a JetBrains Mono Nerd
Font installed in Windows is used; with neither, menus use Segoe UI with plain
letters instead of icons. keyspace.log says which at every start.
