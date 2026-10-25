import type { Asset, Project } from "../model";

export const assetUsage = (asset: Asset) => asset.usage ?? "material";
export const isStoryAsset = (asset: Asset) => assetUsage(asset) === "asset";
export function setAssetUsage(
  project: Project,
  id: string,
  usage: "asset" | "material",
): Project {
  return {
    ...project,
    assets: project.assets.map((a) => (a.id === id ? { ...a, usage } : a)),
  };
}

export const isLibraryAsset = (asset: Asset) =>
  !asset.annotationSourceId &&
  (asset.inLibrary ?? (!asset.generated && !asset.attachmentOnly));

// Removed library items remain addressable by existing edits, but not new picks.
export const isSelectableAsset = (asset: Asset) =>
  !asset.annotationSourceId && !asset.missing && asset.inLibrary !== false;

export function collectAsset(project: Project, asset: Asset): Project {
  const existing = project.assets.find((a) => a.id === asset.id);
  return {
    ...project,
    removedAssetIds: project.removedAssetIds?.filter((id) => id !== asset.id),
    assets: existing
      ? project.assets.map((a) =>
          a.id === asset.id ? { ...a, inLibrary: true } : a,
        )
      : [...project.assets, { ...asset, inLibrary: true }],
  };
}

// Membership is separate from references in shots, conversations and the timeline.
export function uncollectAsset(project: Project, id: string): Project {
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id === id ? { ...a, inLibrary: false } : a,
    ),
  };
}
