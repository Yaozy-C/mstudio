import type { Project } from "../model";
import { productionItems } from "./items";
import type { ProductionTask } from "./types";

export const resultSize = (kind: ProductionTask["kind"]) => ({
  width: kind === "video" ? 260 : 250,
  height: 350,
});

// Pending runs reserve space before media exists. The current run's own slot
// stays available when importing its result; terminal runs leave only cards.
export function resultObstacles(project: Project, task: ProductionTask) {
  return [
    ...productionItems(project),
    ...Object.values(project.production?.drafts ?? {})
      .filter(
        (run) =>
          run.key !== task.key &&
          run.turnId &&
          run.position &&
          !["COMPLETED", "FAILED", "CANCELLED"].includes(run.status ?? "") &&
          !run.resultAssetId &&
          !run.resultAssetIds?.length,
      )
      .map((run) => ({ ...run.position!, ...resultSize(run.kind) })),
  ];
}

export function overlapsResult(
  position: { x: number; y: number },
  size: { width: number; height: number },
  other: { x: number; y: number; width: number; height: number },
) {
  return (
    other.x < position.x + size.width + 32 &&
    other.x + other.width + 32 > position.x &&
    other.y < position.y + size.height + 32 &&
    other.y + other.height + 32 > position.y
  );
}

// Preserve the reserved location unless another card or task occupies it.
export function freeResultPosition(
  project: Project,
  task: ProductionTask,
  preferred: { x: number; y: number },
  size = resultSize(task.kind),
) {
  const position = { ...preferred };
  const obstacles = resultObstacles(project, task);
  while (obstacles.some((n) => overlapsResult(position, size, n)))
    position.y += size.height + 45;
  return position;
}
