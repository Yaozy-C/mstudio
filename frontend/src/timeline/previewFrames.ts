export function decodePreviewFrame(buffer: ArrayBuffer) {
  if (!buffer.byteLength) return null;
  if (buffer.byteLength < 16) throw new Error("预览帧数据不完整");
  const header = new DataView(buffer);
  const sequence = header.getUint32(0, true);
  const width = header.getUint32(4, true);
  const height = header.getUint32(8, true);
  if (
    !width ||
    !height ||
    width > 3840 ||
    height > 3840 ||
    buffer.byteLength !== 16 + width * height * 4
  )
    throw new Error("预览帧尺寸无效");
  return {
    sequence,
    position: header.getUint32(12, true),
    width,
    height,
    pixels: new Uint8ClampedArray(buffer, 16),
  };
}
// One request in flight; no accumulating frame queue while the UI is busy.
export function startPreviewFrames(
  canvas: HTMLCanvasElement,
  read: (last: number) => Promise<ArrayBuffer>,
  fail: (e: unknown) => void,
  accept: (position: number) => boolean = () => true,
  ready: () => void = () => {},
) {
  let alive = true,
    scheduled = 0,
    last = 0,
    presented = false;
  const context = canvas.getContext("2d");
  if (!context) {
    fail(new Error("无法创建预览画布"));
    return () => {};
  }
  const tick = async () => {
    if (!alive) return;
    try {
      if (!document.hidden) {
        const data = await read(last);
        if (!alive) return;
        const frame = decodePreviewFrame(data);
        // Recheck after the async read: a seek may have started while this
        // request was in flight. Never paint preroll/old-position frames.
        if (frame && accept(frame.position)) {
          if (canvas.width !== frame.width || canvas.height !== frame.height) {
            canvas.width = frame.width;
            canvas.height = frame.height;
          }
          context.putImageData(
            new ImageData(frame.pixels, frame.width, frame.height),
            0,
            0,
          );
          last = frame.sequence;
          if (!presented) {
            presented = true;
            ready();
          }
        }
      }
      if (alive) scheduled = requestAnimationFrame(tick);
    } catch (e) {
      if (alive) fail(e);
    }
  };
  scheduled = requestAnimationFrame(tick);
  return () => {
    alive = false;
    cancelAnimationFrame(scheduled);
  };
}
