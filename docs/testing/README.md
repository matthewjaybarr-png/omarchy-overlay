# Windows testing hand-off

**This hand-off lives on the branch `testing-handoff`** (from 2026-11-25 21:20 UTC). It is
never merged anywhere. Code and builds stay on `forge/quirky-dirac-tx8mpd`, which no longer
carries `docs/testing`. Fetch `testing-handoff` to read the queue and the conversation, and
push results, screenshots and messages there. Changes to `tests/` still go to
`forge/quirky-dirac-tx8mpd`.

**Work branches only.** This folder never goes to `main` or into a release: remove it
(`git rm -r docs/testing`) in the last commit before the branch is merged. CI fails a
pull request into `main`, and the release workflow fails a tag, while it is here.

Two Claude sessions work on `forge/quirky-dirac-tx8mpd`: one in the cloud
writes the code, one on a Windows PC tests it. They can't message each other, so they
coordinate through these files.

- `queue.md` — written by the cloud session: what to test, by build, with test IDs.
- `results.md` — written by the Windows session: one entry per test run.
- `conversation.md` — written by both: questions, answers and open items. Read it first.

## For the Windows session

1. `git fetch origin testing-handoff` and read `conversation.md`, then `queue.md`.
2. Get the build named there: the **release** artifact of that GitHub Actions run
   (Actions → Release → the run → Artifacts), or build it yourself with
   `cargo build --release --target x86_64-pc-windows-msvc`. Use the Standard zip
   unless a test says Admin.
3. Quit any running Omarchy Overlay first (tray → Exit).
4. Run the tests and add an entry to `results.md` (newest first), using the IDs:

   ```
   ## Build 5 (7fd09c4) — 2026-11-25 15:10, Windows 11 24H2, 2 monitors
   - PASS F1, F2, D1
   - FAIL D3: middle-click opened nothing. Log: "…" (last lines of
     %LOCALAPPDATA%\OmarchyOverlay\omarchy-overlay.log)
   - SKIP V1: no VS Code on this PC
   ```

5. Commit only `docs/testing/` and the VM suites in `tests/`, with `git pull
   --rebase` first, and push to `forge/quirky-dirac-tx8mpd`. Never change `src/` or
   the other code on this branch; describe problems in conversation.md instead.
6. Screenshots: put them in `docs/testing/shots/` named `<build>-<test id>.png` and
   mention them in the entry. Never include anything private on screen.

## For the cloud session

Read `results.md` after every fetch, fix failures, add the fix to `queue.md` as a
retest, and keep `queue.md` ordered newest build first.
