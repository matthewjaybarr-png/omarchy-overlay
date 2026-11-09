#!/usr/bin/env python3
"""Builds the Standard and Admin release zips from one release exe."""
import argparse
import hashlib
import tempfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_EXE = ROOT / 'target/x86_64-pc-windows-msvc/release/OmarchyOverlay.exe'


README = {
    'standard': """Omarchy Overlay {version} - Standard edition

Runs from this folder. Nothing is installed and no administrator rights are needed.
Start OmarchyOverlay.exe. Settings and logs are kept in %LOCALAPPDATA%\\OmarchyOverlay.
If the desktop is ever left changed, run RECOVER.cmd.
Windows running as administrator do not pass keys to it; use the Admin edition for that.
Example config files are in config\\examples.
""",
    'admin': """Omarchy Overlay {version} - Admin edition

Run INSTALL.cmd and accept the administrator prompt. It installs to
%ProgramFiles%\\OmarchyOverlay and starts elevated at sign-in, so its keys also
work in apps running as administrator. Apps you start from it open without
administrator rights.
Uninstall from Settings > Apps or the tray menu; it restores the desktop first.
Settings and logs are kept in %LOCALAPPDATA%\\OmarchyOverlay.
If the desktop is ever left changed, run RECOVER.cmd.
Example config files are in config\\examples.
""",
}


def build(exe: Path, out: Path) -> list[Path]:
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    out.mkdir(parents=True, exist_ok=True)
    zips = []
    for edition in ('standard', 'admin'):
        path = out / f'OmarchyOverlay-{version}-{edition}.zip'
        with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
            z.write(exe, 'OmarchyOverlay.exe')
            z.write(ROOT / 'RECOVER.cmd', 'RECOVER.cmd')
            z.write(ROOT / 'LICENSE', 'LICENSE.txt')
            readme = README[edition].format(version=version).replace('\n', '\r\n')
            z.writestr('README.txt', readme)
            for folder, prefix in ((ROOT / 'assets/fonts', 'fonts'),
                                   (ROOT / 'config/examples', 'config/examples')):
                for file in sorted(folder.rglob('*')):
                    if file.is_file():
                        z.write(file, f'{prefix}/{file.relative_to(folder).as_posix()}')
            if edition == 'admin':
                z.write(ROOT / 'packaging/INSTALL.cmd', 'INSTALL.cmd')
                z.writestr('edition-admin', '')
        zips.append(path)
    return zips


def selftest() -> None:
    with tempfile.TemporaryDirectory() as temp:
        temp = Path(temp)
        exe = temp / 'OmarchyOverlay.exe'
        exe.write_bytes(b'MZ dummy exe')
        standard, admin = build(exe, temp / 'dist')
        version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
        assert standard.name == f'OmarchyOverlay-{version}-standard.zip', standard.name
        assert admin.name == f'OmarchyOverlay-{version}-admin.zip', admin.name
        names = {z: set(zipfile.ZipFile(z).namelist()) for z in (standard, admin)}
        common = {'OmarchyOverlay.exe', 'RECOVER.cmd', 'README.txt', 'LICENSE.txt',
                  'fonts/JetBrainsMonoNerdFont-Regular.ttf', 'config/examples/binds.conf'}
        assert common <= names[standard], common - names[standard]
        assert common <= names[admin], common - names[admin]
        assert not {'INSTALL.cmd', 'edition-admin'} & names[standard], names[standard]
        assert names[admin] - names[standard] == {'INSTALL.cmd', 'edition-admin'}, names[admin] - names[standard]
        digest = lambda z: hashlib.sha256(zipfile.ZipFile(z).read('OmarchyOverlay.exe')).hexdigest()
        assert digest(standard) == digest(admin) == hashlib.sha256(exe.read_bytes()).hexdigest()
        assert zipfile.ZipFile(admin).read('edition-admin') == b''
        readme = lambda z: zipfile.ZipFile(z).read('README.txt').decode()
        assert 'Standard' in readme(standard) and 'INSTALL.cmd' not in readme(standard)
        assert 'INSTALL.cmd' in readme(admin)
    print('package selftest passed')


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, default=DEFAULT_EXE)
    parser.add_argument('--out', type=Path, default=ROOT / 'dist')
    parser.add_argument('--selftest', action='store_true')
    args = parser.parse_args()
    if args.selftest:
        selftest()
        return
    for path in build(args.exe, args.out):
        print(path)


if __name__ == '__main__':
    main()
