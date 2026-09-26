import type { Project } from "../model";
import type { ProductionSource, ProductionTask } from "../production/types";
export type GeneratedJob = {
  id: string;
  status: string;
  requestId?: string;
  shot: ProductionSource;
  error?: string;
  progress?: ProductionTask["progress"];
  asset?: { id: string };
  outputCount?: number;
  assets?: { id: string }[];
};
export function needsTracking(job: GeneratedJob, project: Project) {
  if (
    [
      "SUBMITTING",
      "UNKNOWN",
      "IN_QUEUE",
      "IN_PROGRESS",
      "CANCEL_REQUESTED",
    ].includes(job.status)
  )
    return true;
  const task = Object.values(project.production?.drafts ?? {}).find(
    (t) => t.jobId === job.id || t.submissionId === job.id,
  );
  if (terminalOutputCount(job) > 0)
    return task
      ? task.status !== job.status ||
          (
            task.resultAssetIds ??
            (task.resultAssetId ? [task.resultAssetId] : [])
          ).length < terminalOutputCount(job)
      : (job.assets?.length ?? (job.asset ? 1 : 0)) < terminalOutputCount(job);
  return !!task && task.status !== job.status;
}

export function terminalOutputCount(job: GeneratedJob): number {
  // A completed response with no output must surface an import error.
  if (job.status === "COMPLETED") return Math.max(1, job.outputCount ?? 1);
  return ["FAILED", "CANCELLED"].includes(job.status)
    ? (job.outputCount ?? 0)
    : 0;
}
