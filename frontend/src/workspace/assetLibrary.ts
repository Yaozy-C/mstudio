import type { Asset, Project } from "../model";

export const assetUsage = (asset: Asset) => asset.usage ?? "material";
export const isStoryAsset = (asset: Asset) =>
  !!asset.assetStatus || assetUsage(asset) === "asset";
export const isMaterial = (asset: Asset) =>
  !asset.annotationSourceId &&
  (asset.inMaterials ??
    (isLibraryAsset(asset) &&
      (!isStoryAsset(asset) || (!asset.generated && !asset.attachmentOnly))));
export const storyAssetStatus = (asset: Asset) =>
  asset.assetStatus ??
  (isStoryAsset(asset)
    ? isLibraryAsset(asset)
      ? "confirmed"
      : "candidate"
    : undefined);
export const isConfirmedStoryAsset = (asset: Asset) =>
  storyAssetStatus(asset) === "confirmed";
export function unconfirmStoryAsset(project: Project, id: string): Project {
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id === id && isStoryAsset(a)
        ? {
            ...a,
            inMaterials: isMaterial(a),
            assetStatus: "candidate",
            inLibrary: isMaterial(a),
          }
        : a,
    ),
  };
}
export function setAssetUsage(
  project: Project,
  id: string,
  usage: "asset" | "material",
): Project {
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id === id
        ? {
            ...a,
            inMaterials: isMaterial(a),
            usage,
            assetStatus: usage === "asset" ? "confirmed" : undefined,
            inLibrary: usage === "asset" ? true : a.inLibrary,
          }
        : a,
    ),
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
          a.id === asset.id
            ? {
                ...a,
                inMaterials: true,
                assetStatus:
                  a.assetStatus ??
                  (isStoryAsset(a)
                    ? isConfirmedStoryAsset(a)
                      ? "confirmed"
                      : "candidate"
                    : undefined),
                inLibrary: true,
              }
            : a,
        )
      : [
          ...project.assets,
          {
            ...asset,
            inMaterials: true,
            assetStatus: storyAssetStatus(asset),
            inLibrary: true,
          },
        ],
  };
}

// Membership is separate from references in shots, conversations and the timeline.
export function uncollectAsset(project: Project, id: string): Project {
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id === id
        ? {
            ...a,
            inMaterials: false,
            assetStatus:
              a.assetStatus ??
              (isStoryAsset(a)
                ? isConfirmedStoryAsset(a)
                  ? "confirmed"
                  : "candidate"
                : undefined),
            inLibrary: isConfirmedStoryAsset(a) ? true : false,
          }
        : a,
    ),
  };
}
