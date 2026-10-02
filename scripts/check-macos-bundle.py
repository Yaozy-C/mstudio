#!/usr/bin/env python3
"""Audit a relocated macOS app and exercise its private media tools."""
from pathlib import Path
import json
import os
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

app = Path(sys.argv[1]).resolve()
with (app / 'Contents/Info.plist').open('rb') as stream:
    assert plistlib.load(stream)['LSMinimumSystemVersion'] == '26.0'
exe_dir = app / 'Contents/MacOS'
count = 0
for path in app.rglob('*'):
    if not path.is_file():
        continue
    description = subprocess.check_output(['file', '-b', str(path)], text=True)
    if 'Mach-O' not in description:
        continue
    assert 'arm64' in description and 'x86_64' not in description, path
    count += 1
    commands = subprocess.check_output(['otool', '-l', str(path)], text=True)
    for version in re.findall(r'\bminos\s+([\d.]+)', commands):
        parts = tuple(map(int, version.split('.')))
        assert (parts + (0, 0))[:3] <= (26, 0, 0), (path, version)
    rpaths = re.findall(r'cmd LC_RPATH\n.*?\n\s+path (.*?) \(offset', commands)

    def expand(value):
        return Path(value.replace('@loader_path', str(path.parent))
                    .replace('@executable_path', str(exe_dir)))

    deps = subprocess.check_output(['otool', '-L', str(path)], text=True).splitlines()[1:]
    for line in deps:
        dep = line.strip().split(' (')[0]
        if dep.startswith(('/usr/lib/', '/System/Library/')):
            continue
        # The first dylib record can be its own install ID.
        if path.suffix == '.dylib' and dep == '@rpath/' + path.name:
            continue
        if dep.startswith('@rpath/'):
            candidates = [expand(r) / dep.removeprefix('@rpath/') for r in rpaths]
            candidates.append(app / 'Contents/Resources/gstreamer/lib' / dep.removeprefix('@rpath/'))
        else:
            candidates = [expand(dep)]
        assert any(p.is_file() and p.resolve().is_relative_to(app) for p in candidates), (path, dep)
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)

with tempfile.TemporaryDirectory(prefix='mstudio bundle 中文 ') as temporary:
    work = Path(temporary)
    # Copy the runtime so that the test also catches absolute references to the
    # original app location. No Homebrew path or inherited DYLD/GST settings.
    contents = work / '测试.app/Contents'
    runtime = contents / 'Resources/gstreamer'
    shutil.copytree(app / 'Contents/Resources/gstreamer', runtime)
    env = {'PATH': '/usr/bin:/bin', 'HOME': str(work), 'TMPDIR': str(work),
           'DYLD_PRINT_LIBRARIES': '1'}

    def run(name, *args):
        result = subprocess.run([str(runtime / 'bin' / name), *map(str, args)],
                                env=env, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, result.stderr
        assert '/opt/homebrew/' not in result.stderr, result.stderr
        assert '/usr/local/' not in result.stderr, result.stderr
        return result.stdout

    video = work / '测试 成片.mp4'
    run('ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=s=320x180:r=24:d=1',
        '-f', 'lavfi', '-i', 'sine=frequency=440:duration=1', '-c:v', 'libx264',
        '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest', video)
    info = json.loads(run('ffprobe', '-v', 'error', '-show_streams', '-show_format',
                         '-of', 'json', video))
    assert {s['codec_type'] for s in info['streams']} == {'video', 'audio'}
    assert abs(float(info['format']['duration']) - 1) < 0.15
    frame = work / '预览.png'
    run('ffmpeg', '-v', 'error', '-i', video, '-frames:v', '1', frame)
    assert frame.stat().st_size > 100
    if len(sys.argv) > 2:
        helper = contents / 'MacOS/bundled_media'
        helper.parent.mkdir(parents=True)
        shutil.copy2(sys.argv[2], helper)
        helper.chmod(0o755)
        for line in subprocess.check_output(['otool', '-L', str(helper)], text=True).splitlines()[1:]:
            dep = line.strip().split(' (')[0]
            if dep.startswith(('/opt/homebrew/', '/usr/local/')):
                bundled = runtime / 'lib' / Path(dep).name
                assert bundled.is_file(), dep
                subprocess.run(['install_name_tool', '-change', dep,
                                '@rpath/' + Path(dep).name,
                                str(helper)], check=True, capture_output=True)
        subprocess.run(['codesign', '--force', '--sign', '-', str(helper)],
                       check=True, capture_output=True)
        result = subprocess.run([str(helper), str(video), str(work / '导入素材')],
                                env=env, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0, result.stderr
        assert '/opt/homebrew/' not in result.stderr, result.stderr
        assert '/usr/local/' not in result.stderr, result.stderr
        print(result.stdout.strip())

print(f'Bundle passed: {count} arm64 Mach-O files, internal dependencies, signature, relocated encode/probe/thumbnail')
