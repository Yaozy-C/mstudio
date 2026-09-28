#!/usr/bin/env python3
"""Point the app executable at the staged GStreamer libraries, then re-sign."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[1]
app=root/'desktop/target/release/bundle/macos/Mstudio.app'
exe=app/'Contents/MacOS/mstudio-desktop'
libs=app/'Contents/Resources/gstreamer/lib'
for line in subprocess.check_output(['otool','-L',str(exe)],text=True).splitlines()[1:]:
    dep=line.strip().split(' (')[0]
    if dep.startswith('/opt/homebrew/'):
        name=Path(dep).name
        if not (libs/name).is_file():raise SystemExit(f'Unbundled dependency: {dep}')
        subprocess.run(['install_name_tool','-change',dep,'@executable_path/../Resources/gstreamer/lib/'+name,str(exe)],check=True)
subprocess.run(['codesign','--force','--deep','--sign','-',str(app)],check=True)
