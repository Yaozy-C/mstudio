#!/usr/bin/env python3
"""Real decoder/compositor smoke test, isolated from user projects."""
from pathlib import Path
import subprocess, tempfile, json, os, sys
root=Path(__file__).resolve().parents[1]
sdk=Path(os.environ.get('MLT_SDK',root/'desktop/native/runtime'))
with tempfile.TemporaryDirectory(prefix='mstudio-mlt-') as folder:
    work=Path(folder)
    def run(args,**kwargs):
        result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
        if result.returncode:
            sys.stderr.write(result.stderr.decode(errors='replace'))
            sys.stderr.write(result.stdout.decode(errors='replace'))
            result.check_returncode()
        return result
    for color in ['red','blue']:
        run(['ffmpeg','-v','error','-y','-f','lavfi','-i',f'color={color}:s=320x240:r=30:d=4','-f','lavfi','-i','sine=frequency=440:duration=4','-c:v','libx264','-preset','ultrafast','-c:a','aac','-shortest',str(work/f'{color}.mp4')])
    assets=[dict(id=c,name=c,kind='video',path=str(work/f'{c}.mp4'),preview='',duration=4,width=320,height=240,hasAudio=True) for c in ['red','blue']]
    def clip(name,start,speed,opacity):return dict(id=name,assetId=name,trimIn=0,trimOut=4,speed=speed,volume=0.5,start=start,trackId='v1',opacity=opacity)
    spec=dict(width=320,height=240,fps=30,tracks=[dict(id='v1',kind='video')],captions=[],clips=[clip('red',0,2,1),clip('blue',1,2,0.5)])
    fixture=work/'fixture.json';fixture.write_text(json.dumps(dict(spec=spec,assets=assets)))
    xml=run(['cargo','run','--quiet','--example','mlt_graph',str(fixture),str(work/'audio-cache')],cwd=root).stdout
    graph=work/'graph.mlt';graph.write_bytes(xml)
    env=dict(os.environ,MLT_DATA=str(sdk/'share/mlt'),MLT_REPOSITORY=str(sdk/'lib/mlt'))
    result=run([str(sdk/'bin/melt'),'-repository',str(sdk/'lib/mlt'),str(graph),'-consumer',f'avformat:{work}/result.mp4','vcodec=libx264','acodec=aac','real_time=-1','preset=ultrafast'],env=env)
    assert b'failed to load' not in result.stderr,result.stderr.decode()
    meta=json.loads(run(['ffprobe','-v','error','-show_entries','format=duration','-of','json',str(work/'result.mp4')]).stdout)
    assert abs(float(meta['format']['duration'])-3)<0.1
    import array
    audio=run(['ffmpeg','-v','error','-ss','0.2','-i',str(work/'result.mp4'),'-t','0.5','-vn','-ac','1','-ar','48000','-f','f32le','-']).stdout
    samples=array.array('f',audio)
    hz=sum(a<=0<b for a,b in zip(samples,samples[1:]))/0.5
    assert abs(hz-440)<8, f'Pitch changed: {hz} Hz'
    print('Pitch-preserving 2x audio:',hz,'Hz')
    pixels=[]
    for t in [0.5,1.5,2.5]:
        pixel=run(['ffmpeg','-v','error','-ss',str(t),'-i',str(work/'result.mp4'),'-frames:v','1','-vf','scale=1:1','-pix_fmt','rgb24','-f','rawvideo','-']).stdout
        pixels.append(tuple(pixel))
    r,m,b=pixels
    assert r[0]>200 and r[2]<20,pixels
    assert 90<m[0]<170 and 90<m[2]<170,pixels
    assert b[0]<20 and 90<b[2]<170,pixels
    print('MLT real render passed: 2x speed, overlapping clips, opacity, duration; RGB:',pixels)
