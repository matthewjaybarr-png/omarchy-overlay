# Contributing

Thanks for helping. Bug reports, ideas and pull requests are all welcome.

- **Bugs and ideas:** open an issue with the bug or feature form. For a bug, the end of
  the log (`%LOCALAPPDATA%\OmarchyOverlay\omarchy-overlay.log`) helps most.
- **First contribution:** issues labelled
  [good first issue](https://github.com/matthewjaybarr-png/omarchy-overlay/labels/good%20first%20issue)
  are small and self-contained. Comment on one to say you're taking it.
- **Building and testing:** see [Build and check](README.md#build-and-check). Before you
  open a pull request, run `cargo test` and
  `cargo clippy --target x86_64-pc-windows-msvc -- -D warnings`. The PowerShell desktop
  tests in `tests\` change the desktop, so run them only in a VM or on a spare machine.
- **Pull requests:** keep each one to one change, and say how you tested it. Add a line to
  `CHANGELOG.md` for anything users will notice.

By contributing you agree that your work is released under the project's
[GPL-3.0 license](LICENSE).
