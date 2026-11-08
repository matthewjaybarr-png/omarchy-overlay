# Omarchy Overlay

A keyboard-first tiling window manager for Windows 11, written in Rust. This is the
public project. It is separate from the author's private build: never reuse its
names, paths or settings.

## Build and check

- Linux: `cargo test`; `cargo clippy --target x86_64-pc-windows-msvc -- -D warnings`; release with `cargo xwin build --release --target x86_64-pc-windows-msvc`.
- Windows: `cargo test`; `cargo build --release --target x86_64-pc-windows-msvc`.
- `tests/windows-*.ps1` are desktop integration tests for a disposable Windows VM; `scripts/test_windows.py` runs them through `scripts/vm.py` (set `OVERLAY_VM`, `OVERLAY_VM_USER`, `OVERLAY_VM_GUEST`).

## Layout

- `src/main.rs` is the entry point. `src/windows.rs` and `src/bar.rs` are large: search for a symbol and read the lines around it.
- `tests/` holds the PowerShell integration tests and `scripts/` the VM helpers. `profiles/`, `assets/`, `config/examples/` and `wallpapers/` are shipped resources.
