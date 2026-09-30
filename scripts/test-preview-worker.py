#!/usr/bin/env python3
"""Exercise the actual bundled preview subprocess protocol, including audio and rates."""
import json
import queue
import struct
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'Mstudio.app/Contents/MacOS/mstudio-desktop'


def read_exact(stream, size):
    chunks = bytearray()
    while len(chunks) < size:
        data = stream.read(size - len(chunks))
        if not data:
            raise EOFError('preview worker exited')
        chunks.extend(data)
    return chunks


with tempfile.TemporaryDirectory(prefix='mstudio-worker-test-') as directory:
    folder = Path(directory)
    image = folder / 'image.png'
    audio = folder / 'silent.wav'
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i',
                    'color=red:s=160x90', '-frames:v', '1', str(image)], check=True)
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i',
                    'anullsrc=r=48000:cl=stereo', '-t', '6', str(audio)], check=True)
    plan = dict(width=160, height=90, fps=30, duration=6, audio=str(audio), files=[],
                layers=[dict(path=str(image), start=0, duration=6, trim=0,
                             x=0, y=0, width=160, height=90, opacity=1)])
    worker = subprocess.Popen([str(EXE), '--preview-worker'], stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    events = queue.Queue()

    def reader():
        try:
            while True:
                size = struct.unpack('<I', read_exact(worker.stdout, 4))[0]
                event = json.loads(read_exact(worker.stdout, size))
                size = struct.unpack('<I', read_exact(worker.stdout, 4))[0]
                frame = read_exact(worker.stdout, size)
                if frame:
                    _, width, height, _ = struct.unpack('<IIII', frame[:16])
                    assert len(frame) == 16 + width * height * 4
                events.put(event)
        except Exception as error:
            events.put(error)

    threading.Thread(target=reader, daemon=True).start()

    def send(value):
        data = json.dumps(value).encode()
        worker.stdin.write(struct.pack('<I', len(data)) + data + struct.pack('<I', 0))
        worker.stdin.flush()

    def wait(kind, request_id=None):
        deadline = time.monotonic() + 20
        while True:
            event = events.get(timeout=max(.01, deadline - time.monotonic()))
            if isinstance(event, Exception):
                raise event
            if kind in event and (request_id is None or event[kind]['id'] == request_id):
                result = event[kind] if kind == 'Ready' else event[kind]['result']
                assert 'Ok' in result, result
                return result['Ok']

    next_id = 0

    def control(action, **fields):
        global next_id
        next_id += 1
        send({'Control': dict(id=next_id, command=dict(action=action, **fields))})
        return wait('Reply', next_id)

    try:
        send({'Open': plan})
        wait('Ready')
        for rate in [.25, .5, .75, 1, 1.25, 1.5, 1.75, 2]:
            control('rate', rate=rate)
            assert control('seek', frame=30)['frame'] == 30
            control('play')
            time.sleep(.3)
            paused = control('pause')
            assert not paused['playing'] and paused['phase'] == 'paused', paused
            assert 30 <= paused['frame'] <= 30 + rate * 30 * .3 + 12, paused
            assert paused['frame'] > 30, paused
            print(f'{rate}x: frame {paused["frame"]}, paused and acknowledged', flush=True)
        control('seek', frame=0)
        control('play')
        assert control('rate', rate=.5)['playing']
        control('pause')
        control('close')
        assert worker.wait(timeout=3) == 0
        print('Packaged worker: audio, all rates, seeks, live rate change and shutdown passed')
    finally:
        if worker.poll() is None:
            worker.kill()
            worker.wait()
