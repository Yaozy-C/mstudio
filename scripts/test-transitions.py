#!/usr/bin/env python3
"""Exercise actual FFmpeg transitions, GES preview, timing, and cache reuse."""
from pathlib import Path
import json, subprocess, tempfile
from ges_test_frame import snapshot
root = Path(__file__).resolve().parents[1]
def run(args):
    p = subprocess.run(list(map(str,args)),cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=120)
    if p.returncode: raise RuntimeError(p.stderr.decode(errors='replace'))
    return p.stdout
def rgb(path,t):
    if path.suffix == ".json": path, t = snapshot(path,t), 0
    data=run(['ffmpeg','-v','error','-ss',t,'-i',path,'-frames:v','1','-vf','scale=1:1','-pix_fmt','rgb24','-f','rawvideo','-'])
    assert len(data)==3, (path,t,len(data))
    return list(data)
with tempfile.TemporaryDirectory(prefix='mstudio-transitions-') as directory:
    folder=Path(directory); assets=[]
    for color in ['red','blue']:
        source=folder/f'{color}.mp4'
        run(['ffmpeg','-v','error','-y','-f','lavfi','-i',f'color={color}:s=320x240:r=30:d=2','-c:v','libx264','-pix_fmt','yuv420p',source])
        assets.append(dict(id=color,name=color,kind='video',path=str(source),preview='',duration=2,width=320,height=240,hasAudio=False))
    clips=[dict(id=c,assetId=c,start=i*2,trimIn=0,trimOut=2,speed=1,volume=0,trackId='v1') for i,c in enumerate(['red','blue'])]
    spec=dict(width=320,height=240,fps=30,tracks=[dict(id='v1',kind='video')],clips=clips,captions=[])
    for kind in ['fade','fadeblack','fadewhite','wipeleft','wiperight','slideleft','slideright','smoothleft','smoothright','circleopen','circleclose','dissolve','custom']:
        clips[1]['transition']=dict(fromClipId='red',kind=kind,duration=1)
        if kind=='custom': clips[1]['transition']['design']=dict(mask='radial',center=[.35,.6],feather=.2,
            curve=[[0,0],[.3,.1],[.65,.8],[1,1]],outgoingZoom=1.5,incomingZoom=1.3)
        fixture=folder/'fixture.json'; fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
        cache=folder/kind; xml=folder/'preview.json'
        xml.write_bytes(run(['cargo','run','--quiet','--example','ges_plan',fixture,cache]))
        patches=list(cache.glob('transition-*.mp4')); assert len(patches)==1
        patch=patches[0]; stamp=patch.stat().st_mtime_ns
        run(['cargo','run','--quiet','--example','ges_plan',fixture,cache])
        assert patch.stat().st_mtime_ns==stamp, 'cache regenerated unnecessarily'
        before,mid,after=[rgb(patch,t) for t in [0,.5,.966]]
        assert before[0]>200 and before[2]<30,(kind,before)
        assert after[2]>200 and after[0]<40,(kind,after)
        if kind=='fade': assert mid[0]>80 and mid[2]>80,(kind,mid)
        if kind=='fadeblack': assert max(mid)<90,(kind,mid)
        if kind=='fadewhite': assert min(mid)>170,(kind,mid)
        if kind in ['fade','slideleft','circleopen','circleclose','custom']:
            preview=xml; exported=folder/'export.mp4'
            run(['cargo','run','--quiet','--example','render_fixture',fixture,folder/'render',exported])
            for t in [1,1.6,2,2.4,3]:
                a,b=rgb(preview,t),rgb(exported,t)
                assert max(abs(x-y) for x,y in zip(a,b))<30,(kind,t,a,b)
            length=float(run(['ffprobe','-v','error','-show_entries','format=duration','-of','csv=p=0',exported]))
            assert abs(length-4)<.05,length
        print(kind,'verified',mid,flush=True)
    # Patterned inputs prove motion changes geometry (solid colors cannot).
    pattern=folder/'pattern.png'
    run(['ffmpeg','-v','error','-y','-f','lavfi','-i','testsrc=s=320x240','-frames:v','1',pattern])
    original_assets=[dict(a) for a in assets]
    for a in assets: a.update(kind='image',path=str(pattern),duration=0)
    def pixels(path,t=0):
        if path.suffix == ".json": path, t = snapshot(path,t), 0
        return run(['ffmpeg','-v','error','-ss',t,'-i',path,'-frames:v','1','-vf','scale=320:240','-pix_fmt','rgb24','-f','rawvideo','-'])
    def difference(a,b):
        assert len(a)==len(b)==320*240*3
        return sum(abs(x-y) for x,y in zip(a,b))/len(a)
    original=pixels(pattern)
    for name,design in [('identity',{}),('zoom',dict(outgoingZoom=2,incomingZoom=2)),
                        ('move',dict(outgoingOffset=[.2,0],incomingOffset=[.2,0]))]:
        clips[1]['transition']=dict(fromClipId='red',kind='custom',duration=1,design=design)
        fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
        cache=folder/name
        xml.write_bytes(run(['cargo','run','--quiet','--example','ges_plan',fixture,cache]))
        patch=next(cache.glob('transition-*.mp4'))
        assert difference(original,pixels(patch,0))<5, 'motion must start at identity'
        if name!='identity':
            assert difference(original,pixels(patch,.5))>15, 'motion did not change geometry'
        # Equal transforms on equal images at the midpoint yield one sharp image.
        reference=folder/'reference.png'
        vf={'identity':'null','zoom':'crop=212:160:54:40,scale=320:240','move':'pad=352:240:32:0,crop=320:240:0:0'}[name]
        run(['ffmpeg','-v','error','-y','-i',pattern,'-vf',vf,'-frames:v','1',reference])
        # Exclude the left edge where perspective extends boundary pixels.
        actual=pixels(patch,.5); expected=pixels(reference)
        interior=lambda data: b''.join(data[(y*320+40)*3:(y*320+300)*3] for y in range(20,220))
        error=sum(abs(a-b) for a,b in zip(interior(actual),interior(expected)))/len(interior(actual))
        assert error<12,(name,error)
        preview=xml; exported=folder/'motion-export.mp4'
        run(['cargo','run','--quiet','--example','render_fixture',fixture,folder/'motion-render',exported])
        parity=difference(pixels(preview,2),pixels(exported,2))
        # Raw GES RGBA and encoded export have effect-dependent conversion/scaling
        # errors; apply the same absolute pixel tolerance to each motion variant.
        assert parity<12, ('motion preview/export mismatch',name,parity)
        print(name,'geometry and preview/export verified',round(error,2),'parity',round(parity,2),flush=True)
    assets[:]=original_assets
    # Real handles, mismatched speeds, an image input and cache invalidation.
    image=folder/'still.png'
    run(['ffmpeg','-v','error','-y','-f','lavfi','-i','color=blue:s=320x240','-frames:v','1',image])
    assets[1].update(kind='image',path=str(image),duration=0)
    clips[0].update(trimIn=.4,trimOut=1.4,speed=2)
    clips[1].update(start=.5,trimIn=0,trimOut=1,speed=.5,transition=dict(fromClipId='red',kind='fade',duration=.4))
    fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
    cache=folder/'handles'
    run(['cargo','run','--quiet','--example','ges_plan',fixture,cache])
    patch=next(cache.glob('transition-*.mp4'))
    mid=rgb(patch,.2); assert mid[0]>60 and mid[2]>60,mid
    clips[0]['visual']=dict(brightness=0,contrast=1,saturation=0,temperature=0,effect='none')
    fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
    run(['cargo','run','--quiet','--example','ges_plan',fixture,cache])
    assert len(list(cache.glob('transition-*.mp4')))==2,'grade failed to invalidate transition cache'
    print('source handles, image, mixed speeds, grade invalidation verified',flush=True)
