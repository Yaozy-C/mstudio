import type { Asset } from "../model";
import { isSelectableAsset } from "../workspace/assetLibrary";
import { mediaAdapter } from "../models/adapters";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";

export function supportsReference(asset: Asset, model?: MediaModel) {
  return (
    isSelectableAsset(asset) &&
    ["image", "video"].includes(asset.kind) &&
    (!model ||
      mediaAdapter(model).fields.some(
        (f) => f.kind === asset.kind && f.role === "reference",
      ))
  );
}
export function addTaskReference(
  task: ProductionTask,
  asset: Asset,
  model?: MediaModel,
): Partial<ProductionTask> {
  if (!supportsReference(asset, model))
    throw new Error("此模型不支持该素材作为参考");
  if (task.inputs.some((r) => r.assetId === asset.id)) return {};
  const limit = model?.capabilities?.referenceLimit;
  if (
    limit !== undefined &&
    task.inputs.filter((r) => r.role !== "script").length >= limit
  )
    throw new Error(`最多引用 ${limit} 个素材`);
  return {
    inputs: [
      ...task.inputs,
      {
        key: `asset:${asset.id}`,
        assetId: asset.id,
        role: asset.kind === "video" ? "video-reference" : "reference",
        // A role label alone is not a purpose; the asset name keeps the input
        // traceable until prompt preparation names the subject it supplies.
        purpose: asset.name,
        ...(asset.kind === "video" ? { start: 0, end: asset.duration } : {}),
      },
    ],
  };
}
