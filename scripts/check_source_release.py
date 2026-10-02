"""Check publishable Git files without printing potential secret values."""
from pathlib import Path
import re
import hashlib
import json
import subprocess

root = Path(__file__).resolve().parents[1]
raw = subprocess.check_output(
    ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root
)
files = sorted(set(raw.decode().split('\0')) - {''})
forbidden_parts = {
    'node_modules', 'target', 'dist', 'output', '.idea', '.vscode',
    '_ui-review', 'workflow-review', '__pycache__',
}
forbidden_extensions = {'.mp4', '.mov', '.wav', '.mp3', '.dylib', '.dll', '.so', '.exe', '.zip', '.gz', '.db', '.pem', '.key', '.p12', '.pfx'}
patterns = [
    re.compile(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'),
    re.compile(r'\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b'),
    re.compile(r'\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_-]{24,}|AKIA[A-Z0-9]{16})\b'),
    re.compile(r'(?i)(?:api[_-]?key|secret|password|access[_-]?token)\s*["\']?\s*[:=]\s*["\'][A-Za-z0-9_+/.=-]{32,}["\']'),
    re.compile(r'(?:/Users/|/home/)[A-Za-z0-9._-]+/'),
]
# Shipped effect previews are application resources, not user production media.
# Only manifest-listed MP4s with pinned content and a bounded size qualify.
effect_previews = {}
errors = []
for preview_root in ['frontend/public/effects', 'frontend/public/transition-previews']:
    manifest = root / preview_root / 'sources.json'
    if manifest.exists():
        for entry in json.loads(manifest.read_text(encoding="utf-8")):
            filename = entry.get('file', '')
            if not re.fullmatch(r'[a-z0-9-]+\.mp4', filename):
                errors.append('Effect preview manifest: invalid filename')
                continue
            name = f'{preview_root}/{filename}'
            path = root / name
            expected_size = entry.get('bytes', 0)
            if (name in effect_previews or not path.is_file() or path.is_symlink()
                or not isinstance(expected_size, int) or not 0 < expected_size <= 3 * 1024 * 1024
                or not entry.get('sourcePage') or not entry.get('sourceVideo') or not entry.get('usage')):
                errors.append(f'{name}: invalid effect preview manifest entry')
                continue
            data = path.read_bytes()
            if len(data) != expected_size or hashlib.sha256(data).hexdigest() != entry.get('sha256'):
                errors.append(f'{name}: effect preview content differs from manifest')
                continue
            effect_previews[name] = expected_size
size = 0
for name in files:
    path = root / name
    parts = Path(name).parts
    if (forbidden_parts.intersection(parts)
        or any(p.endswith('.app') for p in parts)
        or (path.suffix in forbidden_extensions and name not in effect_previews)
        or '.sqlite' in path.name
        or (path.name.startswith('.env') and not path.name.endswith('.example'))
        or name.startswith('examples/') and len(parts) > 2
        or name.startswith('desktop/native/') and len(parts) > 3
        or name.startswith('docs/') and name not in {'docs/architecture.md', 'docs/development.md'}):
        errors.append(f'{name}: non-source/private artifact')
    if path.is_symlink():
        errors.append(f'{name}: symlink requires review')
        continue
    if not path.is_file():
        continue
    data = path.read_bytes()
    size += len(data)
    if len(data) > effect_previews.get(name, 2 * 1024 * 1024):
        errors.append(f'{name}: file exceeds 2 MiB')
    try:
        text = data.decode('utf-8')
    except UnicodeDecodeError:
        continue
    for number, line in enumerate(text.splitlines(), 1):
        if any(pattern.search(line) for pattern in patterns):
            errors.append(f'{name}:{number}: potential credential or personal path; inspect locally')
if errors:
    raise SystemExit('\n'.join(errors))
print(f'Source release: {len(files)} files, {size / 1024 / 1024:.2f} MiB; no prohibited artifacts or pattern matches')
