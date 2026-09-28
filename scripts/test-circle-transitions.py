#!/usr/bin/env python3
"""Check every encoded circle frame, short durations, aspect ratios and direction."""
from pathlib import Path
import json
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

def run(args):
    result = subprocess.run(list(map(str, args)), cwd=ROOT, capture_output=True, timeout=120)
    if result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    return result.stdout

with tempfile.TemporaryDirectory(prefix='mstudio-circle-') as directory:
    root = Path(directory)
    for width, height, fps in [(320, 240, 30), (240, 320, 30), (320, 240, 24), (240, 320, 60)]:
        assets = []
        for color in ['red', 'blue']:
            path = root / f'{color}-{width}-{fps}.mp4'
            run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i',
                 f'color={color}:s={width}x{height}:r={fps}:d=2', '-c:v', 'libx264', path])
            assets.append(dict(id=color, name=color, kind='video', path=str(path), preview='',
                               duration=2, width=width, height=height, hasAudio=False))
        for kind in ['circleopen', 'circleclose']:
            for duration in [.05, .1, .5, 1]:
                clips = [dict(id=color, assetId=color, start=i*2, trimIn=0, trimOut=2,
                              speed=1, volume=0, trackId='v1') for i, color in enumerate(['red', 'blue'])]
                clips[1]['transition'] = dict(fromClipId='red', kind=kind, duration=duration)
                spec = dict(width=width, height=height, fps=fps, tracks=[dict(id='v1', kind='video')],
                            clips=clips, captions=[])
                fixture = root / 'fixture.json'
                fixture.write_text(json.dumps(dict(spec=spec, assets=assets)))
                cache = root / f'{width}-{fps}-{kind}-{duration}'
                run(['cargo', 'run', '--quiet', '--example', 'ges_plan', fixture, cache])
                patch = next(cache.glob('transition-*.mp4'))
                raw = run(['ffmpeg', '-v', 'error', '-i', patch, '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'])
                size = width*height*3
                frames = [raw[i:i+size] for i in range(0, len(raw), size)]
                assert len(frames) >= 2
                red = [sum(f[0::3])/(width*height) for f in frames]
                blue = [sum(f[2::3])/(width*height) for f in frames]
                assert red[0] > 240 and blue[0] < 10, (kind, duration, 'first', red, blue)
                assert blue[-1] > 240 and red[-1] < 10, (kind, duration, 'last', red, blue)
                assert all(b >= a-1 for a, b in zip(blue, blue[1:])), (kind, duration, 'reversal')
                for a, b in zip(frames, frames[1:]):
                    # Lossy edge ringing can differ slightly; a visible return to
                    # the previous image across the frame must not pass.
                    backwards = sum(y < x-24 for x, y in zip(a[2::3], b[2::3]))
                    assert backwards/(width*height) < .005, (kind, duration, 'pixel flicker')
                if duration >= .5:
                    mid = frames[len(frames)//2]
                    center = ((height//2)*width+width//2)*3
                    if kind == 'circleopen':
                        assert mid[center+2] > 220 and mid[0] > 220
                    else:
                        assert mid[center] > 220 and mid[2] > 220
                print(width, height, fps, kind, duration, 'all frames verified', flush=True)
