import { bridge } from "../bridge";
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
    width > 1280 ||
    height > 1280 ||
    buffer.byteLength !== 16 + width * height * 4
  )
    throw new Error("预览帧尺寸无效");
  return { sequence, width, height, pixels: new Uint8ClampedArray(buffer, 16) };
}
// One request in flight; no accumulating frame queue while the UI is busy.
export function startPreviewFrames(
  canvas: HTMLCanvasElement,
  token: string,
  fail: (e: unknown) => void,
) {
  let alive = true,
    scheduled = 0,
    last = 0;
  const context = canvas.getContext("2d");
  if (!context) {
    fail(new Error("无法创建预览画布"));
    return () => {};
  }
  const tick = async () => {
    if (!alive) return;
    try {
      if (!document.hidden) {
        const data = await bridge<ArrayBuffer>("native_preview_frame", {
          token,
          last,
        });
        if (!alive) return;
        const frame = decodePreviewFrame(data);
        if (frame) {
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
