#!/usr/bin/env python3
"""Stage MSVC x64 DLLs beside the app, plugins privately, and standalone FFmpeg."""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
if platform.system() != 'Windows' or platform.machine().lower() not in ('amd64', 'x86_64'):
    raise SystemExit('Run on Windows x64 with the MSVC GStreamer SDK')
sdk = Path(os.environ['GSTREAMER_1_0_ROOT_MSVC_X86_64'])
ffmpeg = Path(os.environ['MSTUDIO_FFMPEG_DIR'])
out = root / 'desktop/native/windows-bundle'
if out.exists():
    shutil.rmtree(out)
out.mkdir(parents=True)


def copy(source, relative):
    target = out / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


# DLLs must be next to the EXE: static imports load before Rust main() executes.
for source in (sdk / 'bin').glob('*.dll'):
    copy(source, source.name)
if not list(out.glob('*ges-1.0*.dll')):
    raise SystemExit('SDK is missing the GES runtime DLL')
plugins = ('coreelements typefindfunctions playback encoding app isomp4 libav '
           'png jpeg videoconvertscale videofilter audiofx imagefreeze videorate '
           'compositor audioconvert audioresample audiomixer audiorate volume '
           'autodetect wasapi nle ges videotestsrc audiotestsrc rawparse '
           'matroska wavparse flac ogg vorbis opus').split()
for name in plugins:
    candidates = [sdk / 'lib/gstreamer-1.0' / f'{prefix}gst{name}.dll' for prefix in ('', 'lib')]
    source = next((p for p in candidates if p.is_file()), None)
    if source is None:
        raise SystemExit(f'Missing GStreamer plugin: {name}')
    copy(source, Path('gstreamer/lib/gstreamer-1.0') / source.name)
for name in ('ffmpeg.exe', 'ffprobe.exe'):
    copy(ffmpeg / 'bin' / name, Path('media') / name)
# Copy optional DLLs for custom shared FFmpeg distributions, isolated from GES.
for source in (ffmpeg / 'bin').glob('*.dll'):
    copy(source, Path('media') / source.name)
# Official MSVC SDKs may rely on the redistributable installed on the build PC.
# Ship app-local CRTs from Visual Studio's redistributable directory.
vswhere = Path(os.environ['ProgramFiles(x86)']) / 'Microsoft Visual Studio/Installer/vswhere.exe'
vs = Path(subprocess.check_output([str(vswhere), '-latest', '-products', '*',
    '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
    '-property', 'installationPath'], text=True).strip())
crt_dirs = sorted((vs / 'VC/Redist/MSVC').glob('*/x64/Microsoft.VC143.CRT'))
if not crt_dirs:
    raise SystemExit('Visual Studio x64 redistributable CRT not found')
for source in crt_dirs[-1].glob('*.dll'):
    copy(source, source.name)
for label, package in [('gstreamer', sdk), ('ffmpeg', ffmpeg)]:
    for folder in ['share/licenses', 'share/doc']:
        if (package / folder).is_dir():
            shutil.copytree(package / folder, out / 'licenses' / label / folder, dirs_exist_ok=True)
    for pattern in ['COPYING*', 'LICENSE*', 'NOTICE*', 'README*']:
        for source in package.glob(pattern):
            if source.is_file():
                copy(source, Path('licenses') / label / source.name)
(out / 'gstreamer/runtime.json').write_text(json.dumps({
    'gstreamer': subprocess.check_output(['pkg-config', '--modversion', 'gstreamer-1.0'], text=True).strip(),
    'ffmpeg': subprocess.check_output([str(ffmpeg / 'bin/ffmpeg.exe'), '-version'], text=True),
    'plugins': plugins, 'architecture': 'x86_64',
}, indent=2), encoding='utf-8')
resources = {'../skills/': 'skills/'}
for file in sorted(out.rglob('*')):
    if file.is_file():
        resources[file.relative_to(root / 'desktop').as_posix()] = file.relative_to(out).as_posix()
(root / 'desktop/native/windows-resources.json').write_text(
    json.dumps({'bundle': {'resources': resources}}, indent=2), encoding='utf-8')
print(f'Staged Windows media runtime: {len(resources) - 1} files')
