"""Prevent new oversized modules or growth of the initial release's legacy files."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
baseline = json.loads((root / 'scripts/source_size_baseline.json').read_text())
violations = []
for folder in ('src', 'desktop/src', 'frontend/src', 'tests', 'scripts'):
    for path in (root / folder).rglob('*'):
        if path.suffix not in ('.rs', '.ts', '.tsx', '.css', '.py', '.sh'):
            continue
        name = path.relative_to(root).as_posix()
        count = len(path.read_text().splitlines())
        limit = baseline.get(name, 300)
        if count > limit:
            violations.append(f'{name}: {count} lines (limit {limit})')
if violations:
    raise SystemExit('\n'.join(violations))
print(f'Source size: no new violations; {len(baseline)} documented legacy allowances')
