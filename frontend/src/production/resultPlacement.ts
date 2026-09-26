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
  const origin = resultOrigin(project, task).origin;
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
  if (!origin) {
    for (let y = top; y + 350 < bottom; y += 395)
      for (let x = left; x + 250 < right; x += 282)
        if (!occupied(x, y)) return { x, y };
  }
  const position = origin
    ? { x: origin.x + origin.width + 32, y: origin.y }
    : { x: left, y: top };
  while (occupied(position.x, position.y)) position.y += 395;
  return position;
}
