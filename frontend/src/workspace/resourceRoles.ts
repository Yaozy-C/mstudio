import type { ProductionTask } from "../production/types";
import type { Asset, Project } from "../model";
import {
  isSelectableAsset,
  isStoryAsset,
  isMaterial,
  isConfirmedStoryAsset,
} from "./assetLibrary";

export const isPlanningMaterial = (a: Asset, project?: Project) =>
  isMaterial(a) &&
  (a.inMaterials === true ||
    isStoryAsset(a) ||
    !project ||
    !isProductionResult(project, a.id));
export const isShotAsset = (a: Asset) =>
  isConfirmedStoryAsset(a) &&
  isSelectableAsset(a) &&
  ["image", "video", "audio"].includes(a.kind);
export const isShotReference = (p: Project, a: Asset) =>
  isShotAsset(a) ||
  (!!a && p.previsAssets?.some((v) => v.id === a.id && !v.missing) === true);

export function isProductionResult(p: Project, id: string) {
  return (
    (p.previsAssets?.some((a) => a.id === id) ||
      p.nodes.some(
        (n) =>
          n.shot &&
          (n.resultAssetId === id ||
            n.shot.frames?.some((f) => f.assetId === id) ||
            n.shot.takes?.some((t) => t.assetId === id)),
      ) ||
      Object.values(p.production?.drafts ?? {}).some(
        (t) =>
          (!!t.ownerId || !!t.targetNodeId) &&
          t.generationPurpose !== "asset" &&
          (t.resultAssetId === id || t.resultAssetIds?.includes(id)),
      )) === true
  );
}

export function canUseProductionReference(p: Project, a: Asset) {
  return (
    isSelectableAsset(a) &&
    (isShotAsset(a) || (!isStoryAsset(a) && isProductionResult(p, a.id)))
  );
}

export const MATERIAL_REFERENCE_ERROR =
  "此资源尚未确认为镜头资产。请先在资产库确认，再用于镜头制作；原素材会保留。";

export const isDraftTask = (task: ProductionTask) =>
  task.generationPurpose !== "asset" && !task.targetNodeId && !task.ownerId;
export function canUseTaskReference(
  p: Project,
  a: Asset,
  task: ProductionTask,
) {
  return (
    isSelectableAsset(a) &&
    (task.generationPurpose === "asset" ||
      isDraftTask(task) ||
      canUseProductionReference(p, a))
  );
}
