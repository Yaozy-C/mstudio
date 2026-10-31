import type { Asset, Project } from "../model";
import { isStoryAsset, setAssetUsage } from "./assetLibrary";
import { isPlanningMaterial } from "./resourceRoles";

export function storyAssets(project: Project) {
  return project.assets.filter((a) => !a.annotationSourceId && isStoryAsset(a));
}

export function addStoryAsset(project: Project, id: string): Project {
  const asset = project.assets.find(
    (a) => a.id === id && !a.annotationSourceId,
  );
  if (
    !asset ||
    asset.missing ||
    !["image", "video", "audio"].includes(asset.kind)
  )
    return project;
  return setAssetUsage(project, id, "asset");
}

export function matchesAsset(asset: Asset, query: string, kind: string) {
  return (
    (kind === "all" ||
      asset.kind === kind ||
      (kind === "document" && asset.kind === "text")) &&
    asset.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())
  );
}

export function availableStoryMaterials(project: Project) {
  return project.assets.filter(
    (a) =>
      isPlanningMaterial(a, project) &&
      !isStoryAsset(a) &&
      ["image", "video", "audio"].includes(a.kind),
  );
}
