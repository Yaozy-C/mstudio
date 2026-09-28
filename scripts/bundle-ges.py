#!/usr/bin/env python3
"""Stage the macOS GES runtime and its playback plugins without editing system libs."""
from pathlib import Path
import subprocess, shutil, json
root=Path(__file__).resolve().parents[1]
out=root/'desktop/native/ges-bundle'
sdk=Path(subprocess.check_output(['pkg-config','--variable=prefix','gstreamer-1.0'],text=True).strip())
plugins='coreelements typefindfunctions playback encoding app isomp4 libav applemedia png jpeg videoconvertscale videofilter audiofx imagefreeze videorate compositor audioconvert audioresample audiomixer audiorate volume autodetect osxaudio nle ges videotestsrc audiotestsrc rawparse matroska wavparse flac ogg vorbis opus'.split()
queue=[]
def copy(src,dst):
    dst.parent.mkdir(parents=True,exist_ok=True)
    shutil.copy2(src,dst,follow_symlinks=True);dst.chmod(dst.stat().st_mode|0o200);queue.append(dst)
for name in ['libges-1.0.0.dylib','libgstreamer-1.0.0.dylib','libgstapp-1.0.0.dylib','libgstvideo-1.0.0.dylib']:
    copy(sdk/'lib'/name,out/'lib'/name)
for name in plugins:
    src=sdk/'lib/gstreamer-1.0'/f'libgst{name}.dylib'
    if not src.exists():raise SystemExit(f'Required GES plugin missing: {src}')
    copy(src,out/'lib/gstreamer-1.0'/src.name)
    dev=root/'desktop/native/ges-dev-plugins';dev.mkdir(exist_ok=True)
    link=dev/src.name
    if link.is_symlink():link.unlink()
    link.symlink_to(src)
seen=set()
while queue:
    file=queue.pop()
    if file in seen:continue
    seen.add(file)
    for line in subprocess.check_output(['otool','-L',str(file)],text=True).splitlines()[1:]:
        dep=line.strip().split(' (')[0]
        if not dep.startswith('/opt/homebrew/'):continue
        src=Path(dep);dst=out/'lib'/src.name
        if dst not in seen and dst not in queue:copy(src,dst)
        relative='@loader_path/'+('../' if file.parent.name=='gstreamer-1.0' else '')+src.name
        subprocess.run(['install_name_tool','-change',dep,relative,str(file)],check=True,capture_output=True)
    subprocess.run(['install_name_tool','-id','@rpath/'+file.name,str(file)],check=True,capture_output=True)
    subprocess.run(['codesign','--force','--sign','-',str(file)],check=True,capture_output=True)
(out/'runtime.json').write_text(json.dumps({'gstreamer':subprocess.check_output(['pkg-config','--modversion','gstreamer-1.0'],text=True).strip(),'plugins':plugins},indent=2))
licenses=out/'licenses';licenses.mkdir(exist_ok=True)
for source in sdk.glob('COPYING*'):shutil.copy2(source,licenses/source.name)
print(f'Staged {len(seen)} GES libraries in {out}')
