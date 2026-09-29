"""Enforce the 300-line limit on maintained source, tests, rules and configuration."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
files = subprocess.check_output(
    ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root
).decode().split('\0')
source_types = {'.rs', '.ts', '.tsx', '.css', '.py', '.sh', '.js', '.mjs', '.cjs',
                '.json', '.md', '.html', '.toml', '.yml', '.yaml', '.cpp', '.h'}
violations = []
checked = 0
for name in sorted(set(files) - {''}):
    path = root / name
    if not path.is_file() or path.suffix not in source_types:
        continue
    count = len(path.read_text().splitlines())
    checked += 1
    if count > 300:
        violations.append(f'{name}: {count} lines (limit 300)')
if violations:
    raise SystemExit('\n'.join(violations))
print(f'Source size: {checked} maintained files at most 300 lines; no exemptions')
