"""Install into an isolated test directory, check startup opt-in/out, uninstall and retain data."""
from pathlib import Path
import subprocess
import winreg

root = Path(__file__).resolve().parents[1]
destination = root / 'tools' / 'installer-smoke'
installer = root / 'dist' / 'Lab-Manager-0.1.0-alpha.1-windows-x64-setup.exe'
uninstall_key = r'Software\Microsoft\Windows\CurrentVersion\Uninstall\{24890BC1-15A7-46AD-9EC2-FD591EF64316}_is1'
run_key = r'Software\Microsoft\Windows\CurrentVersion\Run'
try:
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, uninstall_key):
        raise RuntimeError('An installed copy already exists. Use a clean Windows test user.')
except FileNotFoundError:
    pass
destination.mkdir(parents=True, exist_ok=True)

def install(tasks):
    subprocess.run([str(installer), '/VERYSILENT', '/SUPPRESSMSGBOXES', '/SP-', '/NORESTART', '/NOICONS',
                    f'/DIR={destination}', f'/TASKS={tasks}', f'/LOG={destination.parent / "installer-smoke.log"}'], check=True, timeout=120)

install('startup')
assert (destination / 'lab-manager.exe').is_file()
assert (destination / 'licenses' / 'THIRD-PARTY-NOTICES.txt').is_file()
assert (destination / 'TERMS.txt').is_file()
with winreg.OpenKey(winreg.HKEY_CURRENT_USER, run_key) as key:
    value, _ = winreg.QueryValueEx(key, 'LabManager')
    assert str(destination / 'lab-manager.exe') in value
print('Install and Windows startup opt-in passed')
install('')
with winreg.OpenKey(winreg.HKEY_CURRENT_USER, run_key) as key:
    try:
        winreg.QueryValueEx(key, 'LabManager')
        raise AssertionError('Startup entry was not removed')
    except FileNotFoundError:
        pass
print('Upgrade and Windows startup opt-out passed')
sentinel = destination / 'data' / 'retention-test.txt'
sentinel.parent.mkdir(exist_ok=True)
sentinel.write_text('Disposable installer smoke-test data', encoding='utf-8')
subprocess.run([str(destination / 'unins000.exe'), '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART'], check=True, timeout=120)
assert sentinel.is_file()
assert not (destination / 'lab-manager.exe').exists()
print('Uninstall and data retention passed')
