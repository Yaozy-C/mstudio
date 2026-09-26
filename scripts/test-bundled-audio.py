#!/usr/bin/env python3
"""Initialize bundled SDL audio in an isolated process without Homebrew search paths."""
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import shutil

root = Path(__file__).resolve().parents[1]
bundle = Path(sys.argv[1]) if len(sys.argv) > 1 else root / 'desktop/native/bundle'
lib = bundle / 'lib'
sdl2 = lib / 'libSDL2-2.0.0.dylib'
if not sdl2.is_file():
    sys.exit(f'Missing bundled SDL2: {sdl2}')
requires_sdl3 = b'libSDL3.dylib' in sdl2.read_bytes()
if requires_sdl3 and not (lib / 'libSDL3.dylib').is_file():
    sys.exit('SDL2 compatibility library requires bundled libSDL3.dylib')
with tempfile.TemporaryDirectory(prefix='mstudio-audio-') as folder:
    isolated = Path(folder)
    shutil.copy2(sdl2, isolated / sdl2.name)
    if requires_sdl3:
        for path in lib.glob('libSDL3*.dylib'):
            shutil.copy2(path, isolated / path.name)
    env = {key: value for key, value in os.environ.items() if not key.startswith('DYLD_')}
    env['SDL_AUDIODRIVER'] = 'dummy'
    code = '''
import ctypes, sys
sdl = ctypes.CDLL(sys.argv[1])
sdl.SDL_Init.argtypes = [ctypes.c_uint32]
sdl.SDL_Init.restype = ctypes.c_int
sdl.SDL_GetError.restype = ctypes.c_char_p
if sdl.SDL_Init(0x10) != 0:
    raise RuntimeError(sdl.SDL_GetError().decode())
sdl.SDL_Quit()
print('Bundled SDL audio initialization passed')
'''
    subprocess.run([sys.executable, '-c', code, str(isolated / sdl2.name)],
                   cwd=isolated, env=env, check=True, timeout=30)
