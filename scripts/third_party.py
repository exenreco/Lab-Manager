"""Collect Cargo dependency license declarations and bundled notices offline."""
import json
from pathlib import Path
import subprocess

metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], text=True))
parts = ['Lab Manager third-party dependency notices\nSee also the included icon license files.\n']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['name'] == 'lab-manager':
        continue
    parts.append(f"\n{'=' * 72}\n{package['name']} {package['version']}\nLicense: {package.get('license') or 'See included license text'}\nRepository: {package.get('repository') or ''}\n")
    root = Path(package['manifest_path']).parent
    files = [p for p in root.iterdir() if p.is_file() and p.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT', 'UNLICENSE'))]
    if package.get('license_file'):
        path = root / package['license_file']
        if path.is_file() and path not in files:
            files.append(path)
    for path in sorted(files):
        parts.append(f'\n--- {path.name} ---\n' + path.read_text(encoding='utf-8', errors='replace'))
for path in sorted(Path('assets/icons').glob('*-LICENSE')):
    parts.append(f'\n--- Icon license: {path.name} ---\n' + path.read_text(encoding='utf-8'))
Path('dist').mkdir(exist_ok=True)
Path('dist/THIRD-PARTY-NOTICES.txt').write_text('\n'.join(parts), encoding='utf-8')
print(f"Collected notices for {len(metadata['packages']) - 1} dependencies")
