import type { Project } from "../model";
import { productionItems } from "./items";
import type { ProductionTask } from "./types";

// Product reference assets can appear in many shots. Only actual frame/take
// sources or an explicit edit target establish ownership.
export function resultOrigin(project: Project, task: ProductionTask) {
  const items = productionItems(project);
  const explicit = task.inputs.filter(
    (r) => r.role === "edit" || r.role === "first-frame",
  );
  const exact = items.filter((item) =>
    explicit.some((r) => r.key === item.key),
  );
  const candidates = exact.length
    ? exact
    : items.filter((item) =>
        explicit.some((r) => r.assetId && r.assetId === item.assetId),
      );
  const origin = candidates.length === 1 ? candidates[0] : undefined;
  return { ownerId: task.ownerId, origin };
}
