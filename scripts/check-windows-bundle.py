#!/usr/bin/env python3
"""Install the NSIS package into a fresh path and test without SDK PATH entries."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

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
