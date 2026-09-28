#!/usr/bin/env python3
"""Compare real native-preview and export frames for basic effects and grading."""
from pathlib import Path
import json
import subprocess
import tempfile
from ges_test_frame import snapshot

root = Path(__file__).resolve().parents[1]
def run(args):
    p = subprocess.run([str(a) for a in args], cwd=root,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=90)
    if p.returncode: raise RuntimeError(p.stderr.decode(errors='replace'))
    return p.stdout

def pixels(path):
    position = 0.3
    if path.suffix == ".json": path, position = snapshot(path,position), 0
    return run(['ffmpeg','-v','error','-ss',position,'-i',path,'-frames:v','1','-vf','scale=160:120','-pix_fmt','rgb24','-f','rawvideo','-'])

with tempfile.TemporaryDirectory(prefix='mstudio-effects-') as directory:
    folder = Path(directory)
    source = folder / 'source.mp4'
    run(['ffmpeg','-v','error','-y','-f','lavfi','-i','testsrc2=s=320x240:r=30:d=1',
         '-c:v','libx264','-pix_fmt','yuv420p',source])
    assets=[dict(id='v',name='test',kind='video',path=str(source),preview='',duration=1,width=320,height=240,hasAudio=False)]
    baseline = None
    for effect in ['none','grayscale','sepia','blur','vignette','grade','custom-grade']:
        visual = dict(effect=effect if effect not in ['grade','custom-grade'] else 'none',brightness=0,contrast=1,saturation=1,temperature=0)
        if effect=='grade': visual.update(brightness=.12,contrast=1.1,saturation=.6,temperature=.4)
        if effect=='custom-grade': visual['grade']=dict(exposure=.4,shadows=25,highlights=-30,vibrance=20,tint=8,
            curves=[[.18,.5,.8],[.25,.5,.75],[.25,.5,.75],[.25,.5,.75]],
            hsl=[[0,-15,0],[0,0,0],[0,0,0],[10,-25,10],[0,0,0],[-15,20,-10],[0,0,0],[0,0,0]],
            wheels=[[220,12,0],[0,0,0],[45,10,0]])
        clip=dict(id='v',assetId='v',start=0,trimIn=0,trimOut=1,speed=1,volume=0,trackId='v1',visual=visual)
        spec=dict(width=320,height=240,fps=30,tracks=[dict(id='v1',kind='video')],clips=[clip],captions=[])
        fixture=folder/'fixture.json';fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
        xml=folder/'preview.json';xml.write_bytes(run(['cargo','run','--quiet','--example','ges_plan',fixture,folder/'cache']))
        preview=xml
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
