"""Package a native Linux or macOS build; execute on the corresponding runner."""
import hashlib
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import tarfile
import tomllib

version = tomllib.loads(Path('Cargo.toml').read_text())['package']['version']
system = platform.system()
arch = {'arm64': 'aarch64', 'AMD64': 'x86_64'}.get(platform.machine(), platform.machine())
dist = Path('dist')
dist.mkdir(exist_ok=True)
stage = Path('target/native-package')
stage.mkdir(parents=True, exist_ok=True)
notices = ['LICENSE', 'NOTICE', 'dist/THIRD-PARTY-NOTICES.txt', 'packaging/UNIX_README.txt']
artifacts = []
if system == 'Darwin':
    app = stage / 'Lab Manager.app'
    binary = app / 'Contents/MacOS'
    resources = app / 'Contents/Resources'
    binary.mkdir(parents=True, exist_ok=True)
    resources.mkdir(parents=True, exist_ok=True)
    shutil.copy2('target/release/lab-manager', binary / 'lab-manager')
    for notice in notices:
        shutil.copy2(notice, resources)
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
        'CFBundleName': 'Lab Manager', 'CFBundleDisplayName': 'Lab Manager',
        'CFBundleIdentifier': 'com.exenreco.lab-manager', 'CFBundleExecutable': 'lab-manager',
        'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': version.split('-')[0],
        'CFBundleVersion': '1', 'LSMinimumSystemVersion': '12.0', 'NSHighResolutionCapable': True,
        'NSHumanReadableCopyright': 'Copyright 2026 Exenreco Bell. Apache License 2.0.',
    }))
    subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    applications = stage / 'Applications'
    if not applications.exists():
        applications.symlink_to('/Applications')
    target = dist / f'Lab-Manager-{version}-macos-{arch}.dmg'
    subprocess.run(['hdiutil', 'create', '-volname', 'Lab Manager', '-srcfolder', str(stage), '-ov', '-format', 'UDZO', str(target)], check=True)
    artifacts.append(target)
else:
    app = stage / 'lab-manager'
    app.mkdir(exist_ok=True)
    shutil.copy2('target/release/lab-manager', app / 'lab-manager')
    for notice in notices:
        shutil.copy2(notice, app)
    target = dist / f'Lab-Manager-{version}-linux-{arch}.tar.gz'
    with tarfile.open(target, 'w:gz') as archive:
        archive.add(app, arcname='lab-manager')
    artifacts.append(target)
    deb = Path('target/debian-package')
    (deb / 'DEBIAN').mkdir(parents=True, exist_ok=True)
    (deb / 'usr/bin').mkdir(parents=True, exist_ok=True)
    (deb / 'usr/share/applications').mkdir(parents=True, exist_ok=True)
    (deb / 'usr/share/doc/lab-manager').mkdir(parents=True, exist_ok=True)
    shutil.copy2('target/release/lab-manager', deb / 'usr/bin/lab-manager')
    for notice in notices:
        shutil.copy2(notice, deb / 'usr/share/doc/lab-manager')
    (deb / 'DEBIAN/control').write_text(f'Package: lab-manager\nVersion: {version.replace("-", "~", 1)}\nArchitecture: amd64\nMaintainer: Exenreco Bell\nDepends: libdbus-1-3, libssl3t64, libxkbcommon0, libegl1, libgl1, xdg-desktop-portal, gnome-keyring, openssh-client, curl\nRecommends: xdg-desktop-portal-gtk, libnotify-bin\nDescription: Local device inventory, monitoring and embedded terminals\n')
    (deb / 'usr/share/applications/lab-manager.desktop').write_text('[Desktop Entry]\nType=Application\nName=Lab Manager\nExec=lab-manager\nIcon=utilities-terminal\nTerminal=false\nCategories=System;Network;\n')
    target = dist / f'Lab-Manager-{version}-linux-{arch}.deb'
    subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(deb), str(target)], check=True)
    artifacts.append(target)
    # Headless GUI startup must remain running rather than failing during initialization.
    process = subprocess.Popen(['xvfb-run', '-a', str(app / 'lab-manager')], env=dict(os.environ, LIBGL_ALWAYS_SOFTWARE='1'))
    try:
        process.wait(timeout=8)
        raise RuntimeError(f'GUI exited during smoke test: {process.returncode}')
    except subprocess.TimeoutExpired:
        process.terminate()
        process.wait(timeout=10)
    print('Linux GUI startup smoke test passed')
checksums = '\n'.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}' for path in artifacts) + '\n'
(dist / f'SHA256SUMS-{system.lower()}-{arch}.txt').write_text(checksums)
print('Packaged:', ', '.join(str(path) for path in artifacts))
