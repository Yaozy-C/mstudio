import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";

export function followGeneration(
  canvas: ProductionController | undefined,
  task: ProductionTask,
  kind?: "image" | "video",
) {
  if (!canvas) return;
  const refs = task.resultAssetId
    ? [{ kind: "asset" as const, id: task.resultAssetId }]
    : task.inputs.flatMap((r) =>
        r.assetId ? [{ kind: "asset" as const, id: r.assetId }] : [],
      );
  const next = canvas.act(kind ?? task.kind, [], refs);
  canvas.focusShot(task.targetNodeId ?? task.ownerId);
  return next;
}
