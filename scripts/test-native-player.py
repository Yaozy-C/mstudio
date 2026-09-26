#!/usr/bin/env python3
"""Exercise the real native transport without opening or changing user projects (macOS)."""
from pathlib import Path
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
sdk = Path(os.environ.get('MLT_SDK', root / 'desktop/native/runtime'))
with tempfile.TemporaryDirectory(prefix='mstudio-player-') as folder:
    binary = Path(folder) / 'player-smoke'
    subprocess.run([
        'c++', '-std=c++17', '-pthread', '-I' + str(sdk / 'include/mlt-7'),
        str(root / 'desktop/native/player.cpp'),
        str(root / 'desktop/native/player_test.cpp'),
        '-L' + str(sdk / 'lib'), '-Wl,-rpath,' + str(sdk / 'lib'),
        '-lmlt++-7', '-lmlt-7', '-o', str(binary),
    ], check=True)
    subprocess.run([str(binary), str(sdk)], check=True, timeout=30,
                   env=dict(os.environ, SDL_AUDIODRIVER='dummy'))
