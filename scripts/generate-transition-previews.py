#!/usr/bin/env python3
"""Render original geometric preview cards using Mstudio's actual transition engine.
No external artwork or third-party media. Run from a checkout with FFmpeg and Cargo.
"""
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'frontend/public/transition-previews'
KINDS = ['fade', 'fadeblack', 'fadewhite', 'wipeleft', 'wiperight',
         'slideleft', 'slideright', 'smoothleft', 'smoothright',
         'circleopen', 'circleclose', 'dissolve', 'custom']


def run(args):
    return subprocess.run(list(map(str, args)), cwd=ROOT, check=True,
                          stdout=subprocess.PIPE).stdout


OUTPUT.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='mstudio-transition-cards-') as temp:
    folder = Path(temp)
    assets = []
    for name, bg, accent, x in [('a', '0x184C61', '0xA8DBCD', 45),
                                ('b', '0xBB6044', '0xF9DA9C', 190)]:
        source = folder / f'{name}.mp4'
        vf = (f'drawbox=x={x}:y=36:w=85:h=85:color={accent}:t=fill,'
              f'drawbox=x=0:y=148:w=320:h=52:color={accent}:t=fill,'
              f'drawbox=x=25:y=165:w=90:h=8:color={bg}:t=fill,'
              f'drawbox=x=25:y=180:w=160:h=4:color={bg}:t=fill')
        run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i',
             f'color={bg}:s=320x200:r=30:d=2', '-vf', vf,
             '-c:v', 'libx264', '-pix_fmt', 'yuv420p', source])
        assets.append(dict(id=name, name=name, kind='video', path=str(source),
                           preview='', duration=2, width=320, height=200, hasAudio=False))
    clips = [dict(id=n, assetId=n, start=i*2, trimIn=0, trimOut=2,
                  speed=1, volume=0, trackId='v1') for i, n in enumerate(['a', 'b'])]
    spec = dict(width=320, height=200, fps=30,
                tracks=[dict(id='v1', kind='video')], clips=clips, captions=[])
    for kind in ['none', *KINDS]:
        if kind == 'none':
            patch = folder / 'cut.mp4'
            run(['ffmpeg', '-v', 'error', '-y', '-i', assets[0]['path'],
                 '-i', assets[1]['path'], '-filter_complex',
                 '[0:v]trim=duration=0.5,setpts=PTS-STARTPTS[a];'
                 '[1:v]trim=duration=0.5,setpts=PTS-STARTPTS[b];[a][b]concat=n=2:v=1:a=0',
                 '-an', '-c:v', 'libx264', patch])
        else:
            clips[1]['transition'] = dict(fromClipId='a', kind=kind, duration=1)
            if kind == 'custom':
                clips[1]['transition']['design'] = dict(mask='radial', center=[.35, .6],
                    feather=.2, curve=[[0, 0], [.3, .1], [.65, .8], [1, 1]],
                    outgoingZoom=1.5, incomingZoom=1.3)
            fixture = folder / 'fixture.json'
            fixture.write_text(json.dumps(dict(spec=spec, assets=assets)))
            cache = folder / kind
            run(['cargo', 'run', '--quiet', '--example', 'ges_plan', fixture, cache])
            patch = next(cache.glob('transition-*.mp4'))
        run(['ffmpeg', '-v', 'error', '-y', '-i', patch, '-vf',
             'tpad=start_mode=clone:start_duration=0.4:stop_mode=clone:stop_duration=0.6',
             '-an', '-c:v', 'libx264', '-crf', '24', '-pix_fmt', 'yuv420p',
             '-movflags', '+faststart', OUTPUT / f'{kind}.mp4'])
        run(['ffmpeg', '-v', 'error', '-y', '-ss', '0.5', '-i', patch,
             '-frames:v', '1', OUTPUT / f'{kind}.jpg'])
        print(kind, flush=True)

records = []
for video in sorted(OUTPUT.glob('*.mp4')):
    data = video.read_bytes()
    records.append(dict(file=video.name, bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                        sourcePage='scripts/generate-transition-previews.py',
                        sourceVideo='Original procedural geometry rendered by Mstudio',
                        usage='Original application preview, distributed under the repository MIT license'))
(OUTPUT / 'sources.json').write_text(json.dumps(records, indent=2) + '\n')
