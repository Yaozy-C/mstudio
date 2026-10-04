import type { Project } from "../model";
import type { ProductionTask } from "./types";
import { productionItems } from "./items";
import { resultOrigin } from "./resultOrigin";

// A placement is only a coordinate snapshot, never asset ownership or lineage.
export function resultPlacement(
  project: Project,
  task: ProductionTask,
  size = { width: 900, height: 650 },
) {
  const items = productionItems(project);
  const origin =
    resultOrigin(project, task).origin ??
    items.find(
      (item) => item.kind === "script" && item.ownerId === task.ownerId,
    );
  const view = project.production?.viewport ?? project.viewport;
  const left = (32 - view.x) / view.scale;
  const top = (80 - view.y) / view.scale;
  const right = (size.width - 32 - view.x) / view.scale;
  const bottom = (size.height - 32 - view.y) / view.scale;
  const occupied = (x: number, y: number) =>
    items.some(
      (n) =>
        n.x < x + 282 &&
        n.x + n.width + 32 > x &&
        n.y < y + 382 &&
        n.y + n.height + 32 > y,
    );
  // At overview zoom, the viewport corner can be thousands of world units
  // from the work. Place beside visible content instead of expanding the board.
  const center = { x: (left + right) / 2, y: (top + bottom) / 2 };
  const nearby =
    view.scale < 0.35
      ? items
          .filter(
            (item) =>
              item.x + item.width > left &&
              item.x < right &&
              item.y + item.height > top &&
              item.y < bottom,
          )
          .sort(
            (a, b) =>
              Math.hypot(
                a.x + a.width / 2 - center.x,
                a.y + a.height / 2 - center.y,
              ) -
              Math.hypot(
                b.x + b.width / 2 - center.x,
                b.y + b.height / 2 - center.y,
              ),
          )[0]
      : undefined;
  const anchor = origin ?? nearby;
  if (!anchor) {
    for (let y = top; y + 350 < bottom; y += 395)
      for (let x = left; x + 250 < right; x += 282)
        if (!occupied(x, y)) return { x, y };
  }
  const position = anchor
    ? { x: anchor.x + anchor.width + 32, y: anchor.y }
    : { x: left, y: top };
  while (occupied(position.x, position.y)) position.y += 395;
  return position;
}
