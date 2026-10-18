import type { Asset, Project } from "../model";

/** Resolve a reference without promoting private previs media into the library. */
export const referenceAsset = (
  project: Project,
  id: unknown,
): Asset | undefined =>
  project.assets.find((a) => a.id === id) ??
  project.previsAssets?.find((a) => a.id === id);

export const referenceAssets = (project: Project): Asset[] => [
  ...project.assets,
  ...(project.previsAssets ?? []),
];
