import type { Project } from "../model";
export function restoreProject(snapshot: Project, current: Project): Project {
  const drafts = { ...snapshot.production?.drafts };
  for (const [key, task] of Object.entries(current.production?.drafts ?? {})) {
    if (task.turnId) drafts[key] = task;
  }
  return {
    ...snapshot,
    revision: (current.revision || 0) + 1,
    production: {
      ...snapshot.production,
      models: current.production?.models,
      drafts,
    },
    assets: [
      ...snapshot.assets,
      ...current.assets.filter(
        (a) => !snapshot.assets.some((b) => b.id === a.id),
      ),
    ].filter(
      (a) =>
        snapshot.assets.some((s) => s.id === a.id) ||
        !snapshot.removedAssetIds?.includes(a.id),
    ),
  };
}
