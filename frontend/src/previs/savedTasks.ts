import type { PrevisTaskSummary } from "./PrevisTaskList";
import type { PrevisExport, PrevisWorkspace } from "./types";

/** Workspace exports arrive newest first; older databases may contain duplicates. */
export function latestPrevisVideos(exports: PrevisExport[]): PrevisExport[] {
  const seen = new Set<string>();
  return exports.filter((video) => {
    if (seen.has(video.fileId)) return false;
    seen.add(video.fileId);
    return true;
  });
}

/** Restore completed renders from durable results without inventing live progress. */
export function savedPrevisTasks(
  workspace: PrevisWorkspace,
  session: PrevisTaskSummary[],
): PrevisTaskSummary[] {
  const tasks = new Map<string, PrevisTaskSummary>(
    latestPrevisVideos(workspace.exports).map((video) => [
      video.fileId,
      {
        fileId: video.fileId,
        name:
          workspace.files.find((file) => file.id === video.fileId)?.name ||
          video.asset.name,
        running: false,
        phase: "complete",
        stopping: false,
        error: "",
        lastKind: "render",
      },
    ]),
  );
  for (const task of session) {
    if (
      task.phase &&
      (tasks.has(task.fileId) ||
        workspace.files.some((file) => file.id === task.fileId))
    )
      tasks.set(task.fileId, task);
  }
  return [...tasks.values()];
}
