import type { Asset, Project } from "../model";

export const isLibraryAsset = (asset: Asset) =>
  asset.inLibrary ?? !asset.generated;

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
