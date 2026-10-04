import type { Asset, Project } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
import type { UploadedReference } from "../creation/generationInput";
import {
  canvasSnapshot,
  inputFor,
  mediaReferences,
} from "../production/request";
import { resultPlacement } from "../production/resultPlacement";
import {
  receiveProductionResult,
  jobStatus,
  saveTask,
} from "../production/document";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { terminalOutputCount } from "../workspace/generatedJobTracking";
export function prepareGeneration(
  p: Project,
  taskKey: string,
  model: MediaModel,
  uploaded?: UploadedReference[],
) {
  const task = p.production?.drafts?.[taskKey];
  if (!task) throw new Error("Generation task no longer exists");
  const position = task.position ?? resultPlacement(p, task);
  const snapshot = canvasSnapshot(p, task, position);
  const source = p.nodes.find((n) => n.id === task.ownerId);
  const references = mediaReferences(task);
  return {
    references,
    input: inputFor(p, task, model, uploaded),
    shot: {
      ...(source ?? { title: "画布生成" }),
      references,
      canvasGeneration: snapshot,
      submissionId: task.submissionId,
      projectRevision: p.revision,
    },
  };
}
export function reconcileJob(p: Project, job: GeneratedJob): Project {
  if (!job.shot?.canvasGeneration) return p;
  const current = Object.values(p.production?.drafts ?? {}).find(
    (task) => task.jobId === job.id || task.submissionId === job.id,
  );
  if (!current) return p;
  const received = new Set(
    current.resultAssetIds ??
      (current.resultAssetId ? [current.resultAssetId] : []),
  );
  const count = terminalOutputCount(job);
  const assets = Array.from(
    { length: count },
    (_, i) => job.assets?.[i] ?? (i === 0 ? job.asset : undefined),
  );
  for (const asset of assets)
    if (asset?.id && !received.has(asset.id))
      p = receiveProductionResult(p, asset as Asset, job.shot);
  p = jobStatus(
    p,
    job.id,
    assets.some((a) => !a?.id) ? "RECEIVING" : job.status,
    job.sync?.error ?? job.error,
    job.progress,
    job.requestId,
  );
  const task = Object.values(p.production?.drafts ?? {}).find(
    (t) => t.jobId === job.id || t.submissionId === job.id,
  );
  return task && !!task.trackingPaused !== !!job.sync?.paused
    ? saveTask(p, { ...task, trackingPaused: !!job.sync?.paused })
    : p;
}
