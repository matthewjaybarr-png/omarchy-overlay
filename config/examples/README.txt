Omarchy Overlay config examples
===========================

These are the built-in versions of each config file. Omarchy Overlay refreshes this
examples folder when it starts, so do not edit the files in here.

To change something:
  1. Copy the file you want from this examples folder into the config folder
     one level up (the folder that holds this examples folder).
  2. Open the copy in Notepad and edit it.
  3. In Omarchy Overlay choose Menu > Update > Config > Reload config files.

Files:
  binds.conf          Keybinds. One per line: key, description, command
  menu.json           The Omarchy Overlay menu (Super+Alt+Space)
  webapps.json        Web apps that Super+A and Super+Shift+A bring to the front
  always-float.json   Windows that never tile
  themes\*.toml       Extra color themes (copy into config\themes\)

If a file has a mistake, Omarchy Overlay keeps using the built-in version. The first
line of the menu (Super+Alt+Space) then says how many files have problems; press
Enter on it, or choose Menu > Update > Config > Config file problems, to see each
file name and line number. Delete your copy to go back to the built-in version.

Upgrades never overwrite your copies; only this examples folder is refreshed.
