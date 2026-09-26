export type Bounds = { x: number; y: number; width: number; height: number };
export type Viewport = { width: number; height: number };
export type ResizeAxis = "width" | "height" | "both";
const clamp = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(value, Math.max(min, max)));
export function fitPanel(rect: Bounds, viewport: Viewport): Bounds {
  const width = clamp(rect.width, 260, viewport.width - 24);
  const height = clamp(rect.height, 180, viewport.height - 82);
  return {
    width,
    height,
    x: clamp(rect.x, 12, viewport.width - width - 12),
    y: clamp(rect.y, 70, viewport.height - height - 12),
  };
}
export function resizePanel(
  start: Bounds,
  dx: number,
  dy: number,
  axis: ResizeAxis,
  viewport: Viewport,
): Bounds {
  return {
    ...start,
    width:
      axis === "height"
        ? start.width
        : clamp(start.width + dx, 260, viewport.width - start.x - 12),
    height:
      axis === "width"
        ? start.height
        : clamp(start.height + dy, 180, viewport.height - start.y - 12),
  };
}
export function compactPanel(rect: Bounds, square = false): Bounds {
  const width = square ? 64 : 132;
  return {
    ...rect,
    x: rect.x + rect.width - width,
    width,
    height: square ? 64 : 44,
  };
}
