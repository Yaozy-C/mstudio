#!/usr/bin/env python3
"""Exercise the shipped BGRA shared-memory worker using existing video files.

Usage: test-metal-preview-worker.py executable video.mp4 [video2.mp4 ...]
No generated footage, model calls, project database or WebView are involved.
"""
import json
import mmap
import queue
import struct
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

if len(sys.argv) < 3 or not 1 <= len(sys.argv[2:]) <= 4:
    raise SystemExit(__doc__)
exe = Path(sys.argv[1]).resolve()
sources = [Path(p).resolve(strict=True) for p in sys.argv[2:]]
width, height, fps = 3840, 2160, 30
stride = (16 + width * height * 4 + 63) & ~63
with tempfile.TemporaryDirectory(prefix='mstudio-metal-worker-') as directory:
    mapping = Path(directory) / 'frames'
    mapping.touch(mode=0o600)
    with mapping.open('r+b') as file:
        file.truncate(64 + stride * 3)
        with mmap.mmap(file.fileno(), 0) as shared:
            shared[:28] = struct.pack('<7I', 0x4d535431, width, height, 3, 0, 0, 0)
            plan = dict(width=width, height=height, fps=fps, duration=3,
                        audio=None, files=[], layers=[
                            dict(path=str(p), start=0, duration=3, trim=0, x=0, y=0,
                                 width=width, height=height, opacity=[1, .32, .27, .22][i])
                            for i, p in enumerate(sources)])
            with (Path(directory) / 'stderr.log').open('w+b') as errors:
                process = subprocess.Popen([str(exe), '--preview-worker'],
                                           stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                           stderr=errors)
                events = queue.Queue()
                frame_lengths = set()

                def exact(size):
                    result = bytearray()
                    while len(result) < size:
                        block = process.stdout.read(size - len(result))
                        if not block:
                            raise EOFError('preview worker exited')
                        result.extend(block)
                    return result

                def reader():
                    try:
                        while True:
                            event = json.loads(exact(struct.unpack('<I', exact(4))[0]))
                            packet = exact(struct.unpack('<I', exact(4))[0])
                            if packet:
                                frame_lengths.add(len(packet))
                                assert len(packet) == 16, 'pixels unexpectedly crossed the pipe'
                            events.put((event, packet))
                    except Exception as error:
                        events.put(error)

                threading.Thread(target=reader, daemon=True).start()

                def send(event):
                    data = json.dumps(event).encode()
                    process.stdin.write(struct.pack('<I', len(data)) + data + struct.pack('<I', 0))
                    process.stdin.flush()

                latest_snapshot = None
                frames_before_reply = 0

                def wait(kind, request_id=None, target=None):
                    global latest_snapshot, frames_before_reply
                    deadline = time.monotonic() + 40
                    while True:
                        # A completed seek can publish its snapshot before the
                        # command reply. Retain that frame while awaiting Reply;
                        # the worker sends each sequence's pixels/header once.
                        if kind == 'Snapshot' and latest_snapshot is not None:
                            status, packet = latest_snapshot
                            if struct.unpack('<4I', packet)[3] == target:
                                return status
                        item = events.get(timeout=max(.01, deadline - time.monotonic()))
                        if isinstance(item, Exception):
                            raise item
                        event, packet = item
                        if 'Snapshot' in event and packet:
                            latest_snapshot = (event['Snapshot'], packet)
                            if kind == 'Reply':
                                frames_before_reply += 1
                        if kind not in event:
                            continue
                        if kind == 'Snapshot':
                            if packet and struct.unpack('<4I', packet)[3] == target:
                                return event[kind]
                            continue
                        result = event[kind]
                        if request_id is not None:
                            if result['id'] != request_id:
                                continue
                            result = result['result']
                        assert 'Ok' in result, result
                        return result['Ok']

                next_id = 0

                def control(action, **fields):
                    global next_id
                    next_id += 1
                    send({'Control': dict(id=next_id, command=dict(action=action, **fields))})
                    return wait('Reply', next_id)

                def inspect_paused(target):
                    status = control('seek', frame=target)
                    target = min(target, status['total'] - 1)
                    assert status['frame'] == target
                    wait('Snapshot', target=target)
                    # The acknowledged paused seek has completed its preroll, so
                    # the producer is quiescent while this diagnostic reads pixels.
                    index = struct.unpack_from('<I', shared, 12)[0]
                    assert index < 3
                    offset = 64 + index * stride
                    seq, w, h, position = struct.unpack_from('<4I', shared, offset)
                    assert seq and (w, h, position) == (width, height, target)
                    start = offset + 16 + ((height // 2) * width + width // 2) * 4
                    pixels = shared[start:start + 256]
                    assert any(pixels[0::4]) or any(pixels[1::4]) or any(pixels[2::4])
                    assert all(alpha == 255 for alpha in pixels[3::4])

                try:
                    send({'OpenMetal': dict(plan=plan, frames=dict(path=str(mapping), width=width, height=height))})
                    wait('Ready')
                    inspect_paused(0)
                    inspect_paused(60)
                    for rate in ([.25, .5, .75, 1, 1.25, 1.5, 1.75, 2] if len(sources) == 1 else [.5, 2]):
                        status = control('rate', rate=rate)
                        assert status['rate'] == rate
                        control('play')
                        time.sleep(.12)
                        assert not control('pause')['playing']
                        inspect_paused(30)
                    inspect_paused(89)
                    control('play')
                    assert control('pause')['phase'] in ['paused', 'ended']
                    control('close')
                    assert process.wait(timeout=3) == 0
                    assert frame_lengths == {16}, frame_lengths
                    print(f'Metal worker passed: 4K, {len(sources)} layers, seeks, rates, play/pause, close; pipe frames are metadata only; retained {frames_before_reply} snapshots arriving before replies')
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait()
