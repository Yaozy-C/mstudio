import type { Project } from "../model";

type Bounds = { x: number; y: number; width: number; height: number };
// Overscan is measured in screen pixels, so fast pans do not expose empty edges.
export function visibleItems<T extends Bounds>(
  items: T[],
  view: Project["viewport"],
  size: { width: number; height: number },
  overscan = 240,
): T[] {
  if (size.width <= 0 || size.height <= 0) return [];
  const left = (-view.x - overscan) / view.scale;
  const top = (-view.y - overscan) / view.scale;
  const right = (size.width - view.x + overscan) / view.scale;
  const bottom = (size.height - view.y + overscan) / view.scale;
  return items.filter(
    (item) =>
      item.x < right &&
      item.x + item.width > left &&
      item.y < bottom &&
      item.y + item.height > top,
  );
}
