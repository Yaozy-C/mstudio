import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";

export function followGeneration(
  canvas: ProductionController | undefined,
  task: ProductionTask,
  kind?: "image" | "video",
) {
  if (!canvas) return;
  const result = task.resultAssetId
    ? (canvas.items.find(
        (n) => n.assetId === task.resultAssetId && n.ownerId === task.ownerId,
      ) ?? canvas.items.find((n) => n.assetId === task.resultAssetId))
    : undefined;
  const inputs = task.inputs.flatMap((r) => {
    const item = canvas.items.find(
      (n) => n.key === r.key || (r.assetId && n.assetId === r.assetId),
    );
    return item ? [item.key] : [];
  });
  const next = canvas.act(kind ?? task.kind, result ? [result.key] : inputs);
  canvas.focusShot(task.ownerId);
  return next;
}
