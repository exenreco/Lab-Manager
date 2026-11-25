"""Publish prerelease assets using Git Credential Manager without logging credentials."""
import json
from pathlib import Path
import subprocess
import urllib.request

VERSION = 'v0.1.0-alpha.1'
API = 'https://api.github.com/repos/exenreco/Lab-Manager'

def credentials():
    result = subprocess.run(['git', 'credential', 'fill'], input='protocol=https\nhost=github.com\n\n', capture_output=True, text=True, check=True)
    fields = dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)
    return fields['password']

token = credentials()

def request(url, body=None, content_type='application/json', method=None):
    data = json.dumps(body).encode() if isinstance(body, dict) else body
    req = urllib.request.Request(url, data=data, method=method, headers={
        'Authorization': f'Bearer {token}', 'Accept': 'application/vnd.github+json',
        'User-Agent': 'Lab-Manager-Release', 'Content-Type': content_type,
    })
    with urllib.request.urlopen(req, timeout=180) as response:
        return json.load(response)

if __name__ == '__main__':
    import sys
    if '--inspect' in sys.argv:
        print(json.dumps(request(API + '/releases'), indent=2))
        raise SystemExit()
    release = request(API + '/releases', {
        'tag_name': VERSION, 'target_commitish': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'name': 'Lab Manager 0.1.0 Alpha 1 — Windows', 'prerelease': True, 'draft': True,
        'body': Path('packaging/RELEASE_NOTES.md').read_text(encoding='utf-8'),
    })
    upload = release['upload_url'].split('{')[0]
    for path in sorted(Path('dist').iterdir()):
        if path.is_file():
            asset = request(upload + '?name=' + path.name, path.read_bytes(), 'application/octet-stream')
            print('Uploaded', asset['name'])
    published = request(API + '/releases/' + str(release['id']), {'draft': False}, method='PATCH')
    print(published['html_url'])
