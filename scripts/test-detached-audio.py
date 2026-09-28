#!/usr/bin/env python3
"""Verify detached audio remains audible, aligned and single-gain in the GES audio cache."""
from pathlib import Path
import array
import json
import math
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]

def run(args):
    result = subprocess.run([str(a) for a in args], cwd=root,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
    if result.returncode:
        raise RuntimeError(result.stderr.decode(errors='replace'))
    return result.stdout

def rms(samples):
    return math.sqrt(sum(v * v for v in samples) / max(1, len(samples)))

with tempfile.TemporaryDirectory(prefix='mstudio-detach-') as folder:
    work = Path(folder)
    source = work / 'source.mp4'
    run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i', 'color=red:s=320x240:r=30:d=4',
         '-f', 'lavfi', '-i', 'sine=frequency=440:duration=4',
         '-af', "volume=0:enable='lt(t,1)+gte(t,3)'", '-c:v', 'libx264',
         '-preset', 'ultrafast', '-c:a', 'aac', '-shortest', source])
    assets = [dict(id='v', name='video', kind='video', path=str(source), preview='',
                   duration=4, width=320, height=240, hasAudio=True)]
    for speed in [0.5, 1, 1.56, 2]:
        outputs = []
        for detached, muted in [(False, False), (True, False), (True, True)]:
            clip = dict(id='video', assetId='v', trimIn=0.5, trimOut=3.5, speed=speed,
                        volume=0 if detached else 0.7, start=0.5, trackId='v1')
            tracks = [dict(id='v1', kind='video'), dict(id='a1', kind='audio', muted=muted)]
            clips = [clip]
            if detached:
                clips.append(dict(clip, id='audio', trackId='a1', volume=0.7))
            spec = dict(width=320, height=240, fps=30, tracks=tracks, clips=clips, captions=[])
            fixture = work / 'fixture.json'
            fixture.write_text(json.dumps(dict(spec=spec, assets=assets)))
            plan = json.loads(run(['cargo', 'run', '--quiet', '--example', 'ges_plan', fixture, work / 'cache']))
            output = plan['audio']
            if output is None:
                output = work / 'silence.wav'
                run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=stereo',
                     '-t', plan['duration'], output])
            samples = array.array('f', run(['ffmpeg', '-v', 'error', '-i', output,
                                          '-f', 'f32le', '-ac', '1', '-ar', '48000', '-']))
            outputs.append(samples)
        original, detached, muted = outputs
        assert rms(original) > 0.02, f'{speed}x fixture is silent'
        assert len(original) == len(detached), f'{speed}x duration changed'
        assert rms([a - b for a, b in zip(original, detached)]) < 0.00001, f'{speed}x audio changed after detaching'
        assert rms(muted) < 0.00001, f'{speed}x muted track still audible'
        assert rms(detached[:int(0.4 * 48000)]) < 0.00001, f'{speed}x audio starts before timeline offset'
        print(f'{speed}x: detached PCM matches original timing/gain; mute and timeline offset passed')
