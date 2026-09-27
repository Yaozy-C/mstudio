#!/usr/bin/env python3
"""Exercise transparent still captions beyond frame zero in the real MLT graph."""
from pathlib import Path
import json
import os
import struct
import subprocess
import tempfile
import zlib

ROOT = Path(__file__).resolve().parents[1]
SDK = Path(os.environ.get('MLT_SDK', ROOT / 'desktop/native/runtime'))
ENV = dict(os.environ, MLT_DATA=str(SDK / 'share/mlt'))


def run(args, **kwargs):
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
    if result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    return result.stdout


def png(path):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack(
            '>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + b''.join(
        bytes((255, 255, 255, 255)) if 100 <= x < 220 and 190 <= y < 215
        else bytes(4) for x in range(320)) for y in range(240))
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack(
        '>IIBBBBB', 320, 240, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows))
        + chunk(b'IEND', b''))


with tempfile.TemporaryDirectory(prefix='mstudio-caption-test-') as folder:
    work = Path(folder)
    png(work / 'caption.png')
    run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i',
         'color=red:s=320x240:r=30:d=3', '-c:v', 'libx264',
         '-preset', 'ultrafast', str(work / 'video.mp4')])
    assets = [dict(id=name, name=name, kind=kind, path=str(work / filename),
                   preview='', duration=3, width=320, height=240, hasAudio=False)
              for name, kind, filename in [('v', 'video', 'video.mp4'),
                                            ('c', 'image', 'caption.png')]]
    spec = dict(width=320, height=240, fps=30, tracks=[dict(id='v1', kind='video')],
                clips=[dict(id='clip', assetId='v', trackId='v1', start=0,
                            trimIn=0, trimOut=3, speed=1, volume=1)],
                captions=[dict(id='caption', text='Caption', assetId='c', start=0.5, end=2.5)])
    fixture = work / 'fixture.json'
    fixture.write_text(json.dumps(dict(spec=spec, assets=assets)))
    graph = work / 'graph.mlt'
    graph.write_bytes(run(['cargo', 'run', '--quiet', '--example', 'mlt_graph',
                           str(fixture)], cwd=ROOT))
    def render(path, bounds=()):
        run([str(SDK / 'bin/melt'), '-repository', str(SDK / 'lib/mlt'),
             str(graph), *bounds, '-consumer', f'avformat:{path}',
             'vcodec=png', 'an=1', 'real_time=-1'], env=ENV)
    movie = work / 'render.mov'
    render(movie)
    def check(path, time, caption):
        raw = run(['ffmpeg', '-v', 'error', '-ss', str(time), '-i', str(path),
                   '-frames:v', '1', '-vf', 'scale=320:240', '-pix_fmt', 'rgb24',
                   '-f', 'rawvideo', '-'])
        def pixel(x, y):
            i = (y * 320 + x) * 3
            return tuple(raw[i:i + 3])
        top, bottom = pixel(160, 40), pixel(160, 200)
        assert top[0] > 220 and max(top[1:]) < 25, ('video obscured', top)
        assert (min(bottom) > 230 if caption else
                bottom[0] > 220 and max(bottom[1:]) < 25), (time, bottom)
    for time, visible in [(0.3, False), (0.5, True), (1.8, True),
                          (2.4, True), (2.6, False)]:
        check(movie, time, visible)
    seek = work / 'seek.mov'
    render(seek, ['in=54', 'out=54'])
    check(seek, 0, True)
    print('Caption rendering passed: transparency, full duration, boundaries, direct seek')
