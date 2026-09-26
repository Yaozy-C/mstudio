#!/usr/bin/env python3
"""Compare real native-preview and export frames for basic effects and grading."""
from pathlib import Path
import json
import os
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
sdk = Path(os.environ.get('MLT_SDK', root / 'desktop/native/runtime'))
env = dict(os.environ, MLT_DATA=str(sdk / 'share/mlt'), MLT_REPOSITORY=str(sdk / 'lib/mlt'))
def run(args):
    p = subprocess.run([str(a) for a in args], cwd=root, env=env,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=90)
    if p.returncode: raise RuntimeError(p.stderr.decode(errors='replace'))
    return p.stdout

def pixels(path):
    return run(['ffmpeg','-v','error','-ss','0.3','-i',path,'-frames:v','1','-vf','scale=160:120','-pix_fmt','rgb24','-f','rawvideo','-'])

with tempfile.TemporaryDirectory(prefix='mstudio-effects-') as directory:
    folder = Path(directory)
    source = folder / 'source.mp4'
    run(['ffmpeg','-v','error','-y','-f','lavfi','-i','testsrc2=s=320x240:r=30:d=1',
         '-c:v','libx264','-pix_fmt','yuv420p',source])
    assets=[dict(id='v',name='test',kind='video',path=str(source),preview='',duration=1,width=320,height=240,hasAudio=False)]
    baseline = None
    for effect in ['none','grayscale','sepia','blur','vignette','grade']:
        visual = dict(effect=effect if effect!='grade' else 'none',brightness=0,contrast=1,saturation=1,temperature=0)
        if effect=='grade': visual.update(brightness=.12,contrast=1.1,saturation=.6,temperature=.4)
        clip=dict(id='v',assetId='v',start=0,trimIn=0,trimOut=1,speed=1,volume=0,trackId='v1',visual=visual)
        spec=dict(width=320,height=240,fps=30,tracks=[dict(id='v1',kind='video')],clips=[clip],captions=[])
        fixture=folder/'fixture.json';fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
        xml=folder/'preview.mlt';xml.write_bytes(run(['cargo','run','--quiet','--example','mlt_graph',fixture]))
        preview=folder/'preview.mp4'
        run([sdk/'bin/melt','-repository',sdk/'lib/mlt',xml,'-consumer',f'avformat:{preview}','vcodec=libx264','an=1','real_time=-1','preset=ultrafast'])
        exported=folder/'export.mp4'
        run(['cargo','run','--quiet','--example','render_fixture',fixture,folder/'render',exported])
        a,b=pixels(preview),pixels(exported)
        error=sum(abs(x-y) for x,y in zip(a,b))/len(a)
        assert error<12, (effect,'preview/export mismatch',error)
        if effect=='none': baseline=a
        else:
            change=sum(abs(x-y) for x,y in zip(a,baseline))/len(a)
            assert change>0.8, (effect,'effect not applied',change)
        if effect=='grayscale':
            assert max(abs(a[i]-a[i+1]) for i in range(0,len(a),3))<5
        print(effect,'preview/export mean RGB difference',round(error,2),flush=True)
