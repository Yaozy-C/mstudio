import type { Project } from "../model";
import type { ProductionTask } from "./types";
import { productionItems } from "./items";
import {
  freeResultPosition,
  resultObstacles,
  resultSize,
  overlapsResult,
} from "./resultLayout";
import { resultOrigin } from "./resultOrigin";

// Reserve once when creating the task. Receiving media reuses this snapshot.
export function resultPlacement(
  project: Project,
  task: ProductionTask,
  size = { width: 900, height: 650 },
  view = project.production?.viewport ?? project.viewport,
) {
  const items = productionItems(project);
  const original = resultOrigin(project, task).origin;
  if (original)
    return freeResultPosition(project, task, {
      x: original.x + original.width + 32,
      y: original.y,
    });
  const shot = items.find(
    (item) =>
      item.kind === "script" &&
      item.ownerId === (task.ownerId ?? task.targetNodeId),
  );
  if (shot)
    return freeResultPosition(project, task, {
      x: shot.x + shot.width + (task.kind === "video" ? 640 : 32),
      y: shot.y,
    });

  // Unassigned work is the only case that depends on the viewport.
  const v =
    view &&
    [view.x, view.y, view.scale].every(Number.isFinite) &&
    view.scale > 0
      ? view
      : { x: 0, y: 0, scale: 1 };
  const left = (32 - v.x) / v.scale;
  const top = (80 - v.y) / v.scale;
  const right = (size.width - 32 - v.x) / v.scale;
  const bottom = (size.height - 32 - v.y) / v.scale;
  const card = resultSize(task.kind);
  const obstacles = resultObstacles(project, task);
  // At overview zoom, keep new work beside visible content.
  const nearby =
    v.scale < 0.35
      ? items.find(
          (item) =>
            item.x + item.width > left &&
            item.x < right &&
            item.y + item.height > top &&
            item.y < bottom,
        )
      : undefined;
  if (nearby)
    return freeResultPosition(project, task, {
      x: nearby.x + nearby.width + 32,
      y: nearby.y,
    });
  for (let y = top; y + card.height <= bottom; y += card.height + 45)
    for (let x = left; x + card.width <= right; x += card.width + 32)
      if (!obstacles.some((n) => overlapsResult({ x, y }, card, n)))
        return { x, y };
  return freeResultPosition(project, task, { x: left, y: top });
}
