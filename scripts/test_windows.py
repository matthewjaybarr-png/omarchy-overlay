#!/usr/bin/env python3
"""Run native fixtures sequentially in the existing signed-in Windows test VM."""
import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULTS = ['smoke', 'recovery', 'customization', 'workspaces', 'desktop',
            'picker', 'modules', 'resize', 'dwindle', 'roadmap-effects',
            'native-controls', 'taskbar', 'workspace-menu', 'policy',
            'bar-editor', 'taskbar-modes', 'appearance-integration', 'scrolling', 'scroll-recovery', 'tray', 'themes']
parser = argparse.ArgumentParser(description=__doc__)
# Release suites, run on request: 'standard' (limited token) and 'install' (run elevated).
RELEASES = ['standard', 'install']
parser.add_argument('suites', nargs='*', choices=DEFAULTS + RELEASES)
parser.add_argument('--guest', default=os.environ.get('OVERLAY_VM_GUEST', r'C:\Users\tester\OmarchyOverlay-Test'))
parser.add_argument('--exe', type=pathlib.Path, default=ROOT / 'target/x86_64-pc-windows-msvc/release/OmarchyOverlay.exe')
args = parser.parse_args()
names = args.suites or DEFAULTS
version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
prefix = 'windows11-' + ''.join(version.split('.')[:2]) + (version.split('.')[2] if version.split('.')[2] != '0' else '') + '-'

def vm(*arguments):
    # The guest agent occasionally fails one guest-exec-status call while a fixture is busy;
    # read-only 'ps' polls are retried, pushes and task starts are not.
    for attempt in range(3 if arguments[0] == 'ps' else 1):
        result = subprocess.run(['python3', str(ROOT / 'scripts/vm.py'), *arguments],
                                cwd=ROOT, text=True, capture_output=True, check=False)
        if not result.returncode:
            return result.stdout
        time.sleep(5)
    raise RuntimeError(result.stderr + result.stdout)

def quote(value):
    return "'" + str(value).replace("'", "''") + "'"

# The guest directory must already exist. Never replace a running controller.
processes = vm('ps', " @(Get-Process OmarchyOverlay -ErrorAction SilentlyContinue).Count")
if int(processes.strip()) != 0:
    raise SystemExit('A controller/guard is running; leave it alone and finish its task first.')
print(vm('push', str(args.exe), args.guest + r'\OmarchyOverlay.exe').strip(), flush=True)
# 0.19: every report records the SHA256 of the exe it tested, checked on the guest's copy.
exe_sha256 = hashlib.sha256(args.exe.read_bytes()).hexdigest()
guest_sha256 = vm('ps', f"(Get-FileHash -Algorithm SHA256 -LiteralPath {quote(args.guest + chr(92) + 'OmarchyOverlay.exe')}).Hash").strip().lower()
if guest_sha256 != exe_sha256:
    raise SystemExit(f'The guest exe ({guest_sha256}) is not the one pushed ({exe_sha256}).')
# Suites that run from the guest root expect local config.json mode; a profile left beside the
# exe (for example by a manual probe) silently switches them to portable-profile mode.
stray = args.guest + r'\overlay-profile.json'
print(vm('ps', f"if(Test-Path -LiteralPath {quote(stray)}){{Remove-Item -LiteralPath {quote(stray)};'Removed a stray overlay-profile.json from the test folder.'}}").strip(), flush=True)
if 'tray' in names:
    print(vm('push', str(ROOT / 'profiles/overlay-profile.sample.json'), args.guest + r'\tray-example.json').strip(), flush=True)
if 'themes' in names:
    # An Omarchy-style theme folder (no wallpapers in the repo); the fixture removes it.
    folder = args.guest + r'\config\themes\neon-ridge'
    vm('ps', f'New-Item -ItemType Directory -Force {quote(folder + chr(92) + "backgrounds")}|Out-Null')
    print(vm('push', str(ROOT / 'themes/omarchy/neon-ridge.toml'), folder + r'\colors.toml').strip(), flush=True)
for name, edition in (('standard', 'standard'), ('install', 'admin')):
    if name in names:
        # The release zip from scripts/package.py, unzipped to <guest>\<edition>.
        archive = ROOT / 'dist' / f'OmarchyOverlay-{version}-{edition}.zip'
        if not archive.is_file():
            raise SystemExit(f'{archive} is missing; run scripts/package.py first.')
        target = args.guest + '\\' + edition
        print(vm('push', str(archive), target + '.zip').strip(), flush=True)
        vm('ps', f'Remove-Item -Recurse -Force -LiteralPath {quote(target)} -ErrorAction SilentlyContinue;'
                 f'Expand-Archive -Force -LiteralPath {quote(target + ".zip")} -DestinationPath {quote(target)}')
(ROOT / 'dist').mkdir(exist_ok=True)
for name in names:
    script = args.guest + '\\windows-' + name + '.ps1'
    report = args.guest + '\\' + name + '-result.json'
    print(vm('push', str(ROOT / 'tests' / ('windows-' + name + '.ps1')), script).strip(), flush=True)
    vm('ps', f'if(Test-Path {quote(report)}){{Remove-Item -LiteralPath {quote(report)}}};if(Test-Path {quote(args.guest + chr(92) + "keys-request.txt")}){{Remove-Item {quote(args.guest + chr(92) + "keys-request.txt")}}}')
    vm('ps', f"$tokens=$null;$errors=$null;[void][Management.Automation.Language.Parser]::ParseFile({quote(script)},[ref]$tokens,[ref]$errors);if($errors.Count){{throw ($errors|Out-String)}}")
    # Suites that switch layout leave a per-desktop layout choice behind, which would override the
    # layout the next suite sets. Assumes the test user's profile is C:\Users\<user>.
    choices = rf"C:\Users\{os.environ.get('OVERLAY_VM_USER', 'tester')}\AppData\Local\OmarchyOverlay\workspace-layouts.json"
    vm('ps', f'if(Test-Path -LiteralPath {quote(choices)}){{Remove-Item -LiteralPath {quote(choices)}}}')
    started = time.monotonic()
    task = vm('interactive', script, *(['--elevated'] if name == 'install' else [])).strip()
    print(task, flush=True)
    task = task.split(': ')[-1]
    while time.monotonic() - started < 290:
        # A fixture that needs real (not injected) key presses writes them to keys-request.txt.
        request = args.guest + r'\keys-request.txt'
        state, _, keys = vm('ps', f"(Get-ScheduledTask -TaskName {quote(task)}).State;if(Test-Path {quote(request)}){{Get-Content -Raw {quote(request)};Remove-Item {quote(request)}}}").strip().partition('\n')
        if keys.split():
            print(vm('keys', *keys.split()).strip(), flush=True)
            continue
        # A report can be written before cleanup. The task-state barrier is required.
        if 'Running' not in state and time.monotonic() - started > 3:
            raw = vm('ps', f'if(Test-Path {quote(report)}){{Get-Content -Raw -LiteralPath {quote(report)}}}')
            if not raw.strip():
                print(vm('ps', f'Get-Content {quote(script + ".console.log")} -Tail 35'))
                raise SystemExit('Fixture ended without a report: ' + name)
            rows = json.loads(raw.lstrip('\ufeff').strip())
            rows = rows if isinstance(rows, list) else [rows]
            failures = [row for row in rows if row.get('passed') is not True]
            print(f'{name}: {len(rows)} checks; {len(failures)} failures', flush=True)
            for row in failures:
                print(json.dumps(row), flush=True)
            local = ROOT / 'dist' / (prefix + name + '.json')
            vm('pull', report, str(local))
            if name == 'themes':
                for theme in ('tokyo-night', 'catppuccin-latte', 'neon-ridge'):
                    vm('pull', args.guest + rf'\theme-{theme}.png', str(ROOT / 'dist' / f'theme-{theme}.png'))
            rows.append({'name': 'Executable tested (information)', 'passed': True,
                         'detail': 'OmarchyOverlay.exe SHA256 ' + exe_sha256})
            local.write_text(json.dumps(rows, indent=1) + '\n', encoding='utf-8')
            if failures:
                raise SystemExit(1)
            break
        time.sleep(3)
    else:
        raise SystemExit('Timed out; inspect and let cleanup finish before another suite: ' + name)
print('All requested native suites passed.', flush=True)
