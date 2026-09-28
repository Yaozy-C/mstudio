"""Read a paused production GES frame, for the media regression scripts."""
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[1]
def snapshot(plan, seconds):
    output = Path(plan).with_suffix('.ppm')
    result = subprocess.run(['cargo', 'run', '--quiet', '--manifest-path',
        'desktop/Cargo.toml', '--example', 'ges_frame', str(plan), str(seconds), str(output)],
        cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=90)
    if result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    return output
