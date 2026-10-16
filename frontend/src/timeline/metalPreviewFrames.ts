import { decodePreviewFrame } from "./previewFrames";

export type MetalFrame = {
  sequence: number;
  position: number;
  generation: number;
};
export type MetalBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
  viewportWidth: number;
  viewportHeight: number;
  visible: boolean;
};
// Native views sit above WebView pixels. Hide the surface when DOM overlays
// intersect it; hit testing remains in the WebView even while Metal is visible.
export function metalBounds(
  stage: HTMLElement,
  width: number,
  height: number,
): MetalBounds {
  const box = stage.getBoundingClientRect();
  const availableWidth = Math.max(0, box.width - 2);
  const availableHeight = Math.max(0, box.height - 2);
  const scale = Math.min(availableWidth / width, availableHeight / height);
  const w = width * scale,
    h = height * scale;
  const x = box.left + (box.width - w) / 2;
  const y = box.top + (box.height - h) / 2;
  let visible =
    !document.hidden &&
    stage.isConnected &&
    w > 0 &&
    h > 0 &&
    x >= 0 &&
    y >= 0 &&
    x + w <= innerWidth &&
    y + h <= innerHeight;
  if (visible) {
    const hit = document.elementFromPoint(x + w / 2, y + h / 2);
    visible = !!hit && stage.contains(hit);
    for (const overlay of Array.from(
      document.querySelectorAll(
        '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"], [data-radix-popper-content-wrapper], dialog[open]',
      ),
    )) {
      const rect = overlay.getBoundingClientRect();
      if (
        rect.width &&
        rect.height &&
        rect.right > x &&
        rect.left < x + w &&
        rect.bottom > y &&
        rect.top < y + h
      ) {
        visible = false;
        break;
      }
    }
  }
  return {
    x,
    y,
    width: w,
    height: h,
    viewportWidth: innerWidth,
    viewportHeight: innerHeight,
    visible,
  };
}
export function startMetalPreviewFrames(
  bounds: () => MetalBounds,
  backend: {
    layout: (bounds: MetalBounds) => Promise<void>;
    read: (last: number) => Promise<MetalFrame | null>;
    present: (frame: MetalFrame) => Promise<boolean>;
  },
  fail: (error: unknown) => void,
  accept: (position: number) => boolean,
  ready: () => void,
  fallback?: {
    canvas: HTMLCanvasElement;
    read: (last: number) => Promise<ArrayBuffer>;
  },
) {
  let alive = true,
    scheduled = 0,
    last = 0,
    fallbackLast = 0,
    presented = false,
    previous = "",
    layoutRevision = 0;
  const tick = async () => {
    if (!alive) return;
    try {
      const rect = bounds();
      // Metal sits above the WebView. Paint the same frame in the DOM before
      // hiding it so menus can overlap the video without leaving a blank stage.
      // Pixel IPC is only used while occluded; ordinary playback stays on Metal.
      if (!rect.visible && !document.hidden && fallback) {
        const data = await fallback.read(fallbackLast);
        if (!alive) return;
        const frame = decodePreviewFrame(data);
        if (frame && accept(frame.position)) {
          const { canvas } = fallback;
          const context = canvas.getContext("2d");
          if (!context) throw new Error("无法创建预览画布");
          if (canvas.width !== frame.width || canvas.height !== frame.height) {
            canvas.width = frame.width;
            canvas.height = frame.height;
          }
          context.putImageData(
            new ImageData(frame.pixels, frame.width, frame.height),
            0,
            0,
          );
          fallbackLast = frame.sequence;
          if (!presented) {
            presented = true;
            ready();
          }
        }
      }
      const key = JSON.stringify([rect, devicePixelRatio]);
      if (key !== previous) {
        const revision = layoutRevision;
        await backend.layout(rect);
        if (!alive) return;
        previous = revision === layoutRevision ? key : "";
        // Redraw a paused frame after a resize or occlusion change.
        last = 0;
      }
      if (rect.visible) {
        const frame = await backend.read(last);
        if (!alive) return;
        if (frame && accept(frame.position)) {
          const submitted = await backend.present(frame);
          if (!alive) return;
          if (submitted && accept(frame.position)) {
            last = frame.sequence;
            if (!presented) {
              presented = true;
              ready();
            }
          }
        }
      }
      if (alive) scheduled = requestAnimationFrame(tick);
    } catch (error) {
      if (alive) fail(error);
    }
  };
  scheduled = requestAnimationFrame(tick);
  const hide = () => {
    layoutRevision++;
    previous = "";
    if (alive && document.hidden)
      void backend.layout({ ...bounds(), visible: false }).catch((error) => {
        if (alive) fail(error);
      });
  };
  document.addEventListener("visibilitychange", hide);
  return () => {
    alive = false;
    cancelAnimationFrame(scheduled);
    document.removeEventListener("visibilitychange", hide);
    void backend.layout({ ...bounds(), visible: false }).catch(() => {});
  };
}
