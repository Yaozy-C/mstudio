import type { Asset } from "../model";
import { modelAdapter } from "../models/adapters";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";

export function supportsReference(asset: Asset, model?: MediaModel) {
  return (
    !asset.missing &&
    ["image", "video"].includes(asset.kind) &&
    (!model ||
      modelAdapter(model.plugin, model.endpoint).fields.some(
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
  if (task.inputs.filter((r) => r.assetId).length >= 12)
    throw new Error("最多引用 12 个素材");
  if (asset.kind === "video" && asset.duration < 2)
    throw new Error("视频参考至少需要 2 秒");
  return {
    inputs: [
      ...task.inputs,
      {
        key: `asset:${asset.id}`,
        assetId: asset.id,
        role: asset.kind === "video" ? "video-reference" : "reference",
        purpose: asset.kind === "video" ? "动作参考" : "内容参考",
        ...(asset.kind === "video"
          ? { start: 0, end: Math.min(5, asset.duration) }
          : {}),
      },
    ],
  };
}
