#!/usr/bin/env python3
"""Stage a self-contained macOS MLT runtime; never modify the Homebrew installation."""
from pathlib import Path
import shutil, subprocess, sys, os, json
root=Path(__file__).resolve().parents[1]
sdk=Path(os.environ.get('MLT_SDK',root/'desktop/native/runtime'))
out=root/'desktop/native/bundle'
if sys.platform=='win32':
    for directory in ['bin','lib/mlt','share']:
        shutil.copytree(sdk/directory,out/directory,dirs_exist_ok=True)
    resources={'native/bundle/':'mlt/'}
    for dll in (out/'bin').glob('*.dll'):
        resources[str(dll.relative_to(root/'desktop')).replace('\\','/')]=dll.name
        for mode in ['debug','release']:
            target=root/'desktop/target'/mode;target.mkdir(parents=True,exist_ok=True)
            shutil.copy2(dll,target/dll.name)
    config=root/'desktop/native/windows-resources.json'
    config.write_text(json.dumps({'bundle':{'resources':resources}},indent=2))
    print(f'Windows DLL staging complete. Build Tauri with --config {config}')
    raise SystemExit(0)
if sys.platform!='darwin': raise SystemExit('Unsupported preview platform')
(out/'lib/mlt').mkdir(parents=True,exist_ok=True)
shutil.copytree(sdk/'share',out/'share',dirs_exist_ok=True)
queue=[]
for src in list((sdk/'lib').glob('*.dylib'))+list((sdk/'lib/mlt').glob('*.so')):
    dst=out/'lib'/('mlt' if src.suffix=='.so' else '')/src.name
    shutil.copy2(src,dst,follow_symlinks=True);dst.chmod(dst.stat().st_mode | 0o200);queue.append(dst)
seen=set()
while queue:
    file=queue.pop()
    if file in seen: continue
    seen.add(file)
    # sdl2-compat dlopens SDL3 by name; it is absent from otool's dependency list.
    if file.name.startswith('libSDL2') and b'libSDL3.dylib' in file.read_bytes():
        libdir=Path(subprocess.check_output(['pkg-config','--variable=libdir','sdl3'],text=True).strip())
        src=libdir/'libSDL3.dylib'
        if not src.is_file(): raise SystemExit(f'SDL2 compatibility runtime requires {src}')
        dst=out/'lib/libSDL3.dylib'
        shutil.copy2(src,dst,follow_symlinks=True)
        dst.chmod(dst.stat().st_mode | 0o200)
        queue.append(dst)
    deps=subprocess.check_output(['otool','-L',str(file)],text=True).splitlines()[1:]
    for line in deps:
        dep=line.strip().split(' (')[0]
        if not dep.startswith('/opt/') and not dep.startswith(str(sdk)): continue
        src=Path(dep);dst=out/'lib'/src.name
        if dst not in seen and dst not in queue: shutil.copy2(src,dst);dst.chmod(dst.stat().st_mode | 0o200);queue.append(dst)
        replacement=('@loader_path/../' if file.parent.name=='mlt' else '@loader_path/')+src.name
        subprocess.run(['install_name_tool','-change',dep,replacement,str(file)],check=True,stderr=subprocess.PIPE)
    if file.suffix=='.dylib':
        subprocess.run(['install_name_tool','-id','@rpath/'+file.name,str(file)],check=True,stderr=subprocess.PIPE)
    subprocess.run(['codesign','--force','--sign','-',str(file)],check=True,stderr=subprocess.PIPE)
licenses=out/'licenses';licenses.mkdir(exist_ok=True)
revision=json.loads((root/'scripts/mlt-source.json').read_text())['revision']
for p in (root/f'desktop/native/vendor/mlt-{revision}').glob('COPYING*'): shutil.copy2(p,licenses/p.name)
# Homebrew dependencies can be read-only. Tauri preserves resource modes and
# then cannot overwrite them on the next incremental build.
for p in out.rglob('*'):
    if p.is_file() and not p.is_symlink(): p.chmod(p.stat().st_mode | 0o200)
print(f'Staged {len(seen)} native libraries in {out}')
