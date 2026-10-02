#!/usr/bin/env python3
"""Install the NSIS package into a fresh path and test without SDK PATH entries."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import struct

root = Path(__file__).resolve().parents[1]
release = root / 'desktop/target/release'
installers = list((release / 'bundle/nsis').glob('*-setup.exe'))
if len(installers) != 1:
    raise SystemExit(f'Expected one NSIS installer, found {len(installers)}')
with tempfile.TemporaryDirectory(prefix='mstudio Windows 中文 ') as temporary:
    work = Path(temporary)
    installed = work / '安装目录'
    env = {k: v for k, v in os.environ.items()
           if not k.upper().startswith(('GST_', 'GSTREAMER', 'DYLD_', 'MSTUDIO_', 'PKG_CONFIG'))}
    system = os.environ['SystemRoot']
    env['PATH'] = os.pathsep.join([str(Path(system) / 'System32'), system])
    env['TEMP'] = env['TMP'] = str(work)
    env['GST_REGISTRY_1_0'] = str(work / 'registry.bin')
    subprocess.run([str(installers[0]), '/S', f'/D={installed}'], env=env, check=True, timeout=180)
    exe = installed / 'mstudio-desktop.exe'
    assert exe.is_file(), f'Installer omitted executable: {list(installed.iterdir())}'
    assert (installed / 'skills/ad-team/SKILL.md').is_file(), 'Installer omitted skills'
    assert (installed / 'gstreamer/runtime.json').is_file(), 'Installer omitted runtime'
    tools = installed / 'media'
    video = work / '中文 有声测试.mp4'

    def run(path, *args):
        result = subprocess.run([str(path), *map(str, args)], env=env,
                                capture_output=True, text=True, encoding='utf-8',
                                errors='replace', timeout=90)
        assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
        return result.stdout

    run(tools / 'ffmpeg.exe', '-v', 'error', '-f', 'lavfi', '-i', 'testsrc2=s=320x180:r=24:d=1',
        '-f', 'lavfi', '-i', 'sine=frequency=440:duration=1', '-c:v', 'libx264',
        '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-shortest', video)
    info = json.loads(run(tools / 'ffprobe.exe', '-v', 'error', '-show_streams',
                         '-show_format', '-of', 'json', video))
    assert {s['codec_type'] for s in info['streams']} == {'video', 'audio'}
    assert abs(float(info['format']['duration']) - 1) < 0.15
    helper = installed / 'bundled_media.exe'
    shutil.copy2(release / 'examples/bundled_media.exe', helper)
    print(run(helper, video, work / '素材目录').strip())
    helper.unlink()
    # Test the actual GUI-subsystem executable's binary stdin/stdout worker IPC.
    plan = dict(width=320, height=180, fps=24, duration=1, audio=None, files=[],
                layers=[dict(path=str(video), start=0, duration=1, trim=0,
                             x=0, y=0, width=320, height=180, opacity=1)])
    def packet(value):
        data = json.dumps(value).encode('utf-8')
        return struct.pack('<I', len(data)) + data + struct.pack('<I', 0)
    requests = packet({'Open': plan}) + packet({'Control': {'id': 1, 'command': {'action': 'seek', 'frame': 12}}})
    requests += packet({'Control': {'id': 2, 'command': {'action': 'close'}}})
    result = subprocess.run([str(exe), '--preview-worker'], input=requests,
                            capture_output=True, env=env, timeout=60)
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    events = []
    data = result.stdout
    while data:
        size = struct.unpack('<I', data[:4])[0]
        events.append(json.loads(data[4:4 + size]))
        frame_size = struct.unpack('<I', data[4 + size:8 + size])[0]
        data = data[8 + size + frame_size:]
    assert any('Ok' in event.get('Ready', {}) for event in events), events
    assert any(event.get('Reply', {}).get('id') == 1 and
               event['Reply']['result'].get('Ok', {}).get('frame') == 12 for event in events), events

    # Check the real GUI entrypoint loads its DLLs and stays alive on a clean PATH.
    process = subprocess.Popen([str(exe)], env=env)
    try:
        try:
            code = process.wait(timeout=12)
        except subprocess.TimeoutExpired:
            pass
        else:
            raise AssertionError(f'Installed GUI exited early: {code}')
    finally:
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=15)
    uninstall = installed / 'uninstall.exe'
    assert uninstall.is_file(), 'Missing uninstaller'
    subprocess.run([str(uninstall), '/S', f'_?={installed}'], env=env, check=True, timeout=90)
    assert not exe.exists(), 'Uninstaller left the main executable'
print(f'Windows installed package passed: {installers[0].name}; clean PATH, Unicode paths, media, GES, GUI and uninstall')
