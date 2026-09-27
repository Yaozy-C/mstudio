import type { Project } from "../model";
import { contentChanged } from "./projectRevision";
export function restoreProject(snapshot: Project, current: Project): Project {
  const drafts = { ...snapshot.production?.drafts };
  for (const [key, task] of Object.entries(current.production?.drafts ?? {})) {
    if (task.turnId) drafts[key] = task;
  }
  const restored = {
    ...snapshot,
    production: {
      ...snapshot.production,
      models: current.production?.models,
      drafts: Object.keys(drafts).length ? drafts : snapshot.production?.drafts,
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
  return {
    ...restored,
    revision:
      (current.revision || 0) + Number(contentChanged(current, restored)),
  };
}
