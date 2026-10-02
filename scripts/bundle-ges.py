#!/usr/bin/env python3
"""Stage the Apple Silicon media runtime without modifying installed libraries."""
from pathlib import Path
import json
import os
import platform
import shutil
import subprocess

root = Path(__file__).resolve().parents[1]
out = root / 'desktop/native/ges-bundle'
if platform.system() != 'Darwin' or platform.machine() != 'arm64':
    raise SystemExit('Packaging requires Apple Silicon macOS')
sdk = Path(subprocess.check_output(
    ['pkg-config', '--variable=prefix', 'gstreamer-1.0'], text=True).strip())
plugins = ('coreelements typefindfunctions playback encoding app isomp4 libav '
           'applemedia png jpeg videoconvertscale videofilter audiofx imagefreeze '
           'videorate compositor audioconvert audioresample audiomixer audiorate '
           'volume autodetect osxaudio nle ges videotestsrc audiotestsrc rawparse '
           'matroska wavparse flac ogg vorbis opus').split()
# Generated resources only: remove stale libraries/plugins from previous builds.
if out.exists():
    shutil.rmtree(out)
queue = []
sources = {}
packages = set()


def copy(src, dst):
    src = src.resolve(strict=True)
    if dst in sources:
        if sources[dst] != src:
            raise SystemExit(f'Conflicting runtime libraries: {sources[dst]} and {src}')
        return
    sources[dst] = src
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    dst.chmod(dst.stat().st_mode | 0o200)
    queue.append(dst)
    # Retain the actual installed package's notices and build metadata.
    for parent in src.parents:
        if (parent / 'INSTALL_RECEIPT.json').is_file():
            packages.add(parent)
            break


for name in ['libges-1.0.0.dylib', 'libgstreamer-1.0.0.dylib',
             'libgstapp-1.0.0.dylib', 'libgstvideo-1.0.0.dylib']:
    copy(sdk / 'lib' / name, out / 'lib' / name)
for name in ['ffmpeg', 'ffprobe']:
    source = shutil.which(name)
    if not source:
        raise SystemExit(f'Required media tool missing: {name}')
    copy(Path(source), out / 'bin' / name)
for name in plugins:
    src = sdk / 'lib/gstreamer-1.0' / f'libgst{name}.dylib'
    copy(src, out / 'lib/gstreamer-1.0' / src.name)
    dev = root / 'desktop/native/ges-dev-plugins'
    dev.mkdir(exist_ok=True)
    link = dev / src.name
    if link.is_symlink():
        link.unlink()
    link.symlink_to(src)

while queue:
    file = queue.pop()
    for line in subprocess.check_output(['otool', '-L', str(file)], text=True).splitlines()[1:]:
        dep = line.strip().split(' (')[0]
        if dep.startswith(('/usr/lib/', '/System/Library/')):
            continue
        if not dep.startswith('/'):
            raise SystemExit(f'Unresolved SDK dependency in {file}: {dep}')
        src = Path(dep)
        dst = out / 'lib' / src.name
        # A dylib lists its own install ID; it is not another dependency.
        if src.resolve() == sources[file]:
            continue
        copy(src, dst)
        relative = '@loader_path/' + os.path.relpath(dst, file.parent)
        subprocess.run(['install_name_tool', '-change', dep, relative, str(file)],
                       check=True, capture_output=True)
    if file.suffix == '.dylib':
        subprocess.run(['install_name_tool', '-id', '@rpath/' + file.name, str(file)],
                       check=True, capture_output=True)
    subprocess.run(['codesign', '--force', '--sign', '-', str(file)],
                   check=True, capture_output=True)

licenses = out / 'licenses'
for package in sorted(packages):
    destination = licenses / package.parent.name / package.name
    destination.mkdir(parents=True, exist_ok=True)
    for pattern in ['COPYING*', 'LICENSE*', 'NOTICE*', 'AUTHORS*',
                    'INSTALL_RECEIPT.json', 'sbom.spdx.json']:
        for source in package.glob(pattern):
            if source.is_file():
                shutil.copy2(source, destination / source.name)
metadata = {
    'gstreamer': subprocess.check_output(
        ['pkg-config', '--modversion', 'gstreamer-1.0'], text=True).strip(),
    'ffmpeg': subprocess.check_output([str(out / 'bin/ffmpeg'), '-version'], text=True),
    'plugins': plugins,
    'files': {str(dst.relative_to(out)): str(src) for dst, src in sorted(sources.items())},
    'packages': [str(p) for p in sorted(packages)],
}
(out / 'runtime.json').write_text(json.dumps(metadata, indent=2))
print(f'Staged {len(sources)} media binaries in {out}')
