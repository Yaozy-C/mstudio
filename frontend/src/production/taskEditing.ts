import {
  assertGenerationReferences,
  replaceTaskReferences,
} from "./taskReferences";
import type { Project } from "../model";
import { saveTask } from "./document";
import type { ProductionTask } from "./types";
import { normalizeTaskPrompt } from "./taskPrompt";

export const canEditOriginal = (task: ProductionTask) =>
  !task.jobId &&
  !task.submissionId &&
  (!task.status || task.status === "AWAITING_CONFIRMATION");
export const canRegenerate = (task: ProductionTask) =>
  ["COMPLETED", "FAILED", "CANCELLED"].includes(task.status ?? "");
export function editTaskPrompt(p: Project, key: string, text: string): Project {
  return editTask(p, key, { text });
}
export function editTask(
  p: Project,
  key: string,
  changes: { text?: string; references?: unknown },
): Project {
  const original = p.production?.drafts?.[key];
  if (!original) throw new Error("任务不存在，请重新查询");
  if (changes.text === undefined && changes.references === undefined)
    throw new Error("请提供生成描述或参考素材");
  let task = normalizeTaskPrompt(original);
  if (changes.text !== undefined) {
    if (
      typeof changes.text !== "string" ||
      !changes.text.trim() ||
      changes.text.length > 12000
    )
      throw new Error("请提供完整生成描述（最多 12000 字）");
    task = { ...task, prompt: changes.text.trim() };
  }
  if (changes.references !== undefined) {
    task = replaceTaskReferences(p, task, changes.references);
    assertGenerationReferences(p, task);
  }
  return saveTask(p, task);
}

export function regenerationDraft(task: ProductionTask): ProductionTask {
  if (!canRegenerate(task) && !canEditOriginal(task))
    throw new Error("请先核查原任务状态，再重新生成");
  return {
    ...structuredClone(normalizeTaskPrompt(task)),
    hiddenFromList: false,
    status: "AWAITING_CONFIRMATION",
    jobId: undefined,
    submissionId: undefined,
    requestId: undefined,
    error: undefined,
    trackingPaused: undefined,
    progress: undefined,
    resultAssetId: undefined,
    resultAssetIds: undefined,
  };
}

// Replacing a result preserves its history but revokes its use as generation input.
export function regenerateTask(
  p: Project,
  key: string,
  changes: {
    text?: string;
    references?: unknown;
    keepPreviousResults?: boolean;
  } = {},
): Project {
  const original = p.production?.drafts?.[key];
  if (!original) throw new Error("任务不存在");
  let task = regenerationDraft(original);
  if (changes.text !== undefined) {
    if (
      typeof changes.text !== "string" ||
      !changes.text.trim() ||
      changes.text.length > 12000
    )
      throw new Error("请提供完整生成描述（最多 12000 字）");
    task.prompt = changes.text.trim();
  }
  task = replaceTaskReferences(p, task, changes.references);
  const obsolete = new Set(
    changes.keepPreviousResults
      ? []
      : [
          ...(original.resultAssetIds ?? []),
          ...(original.resultAssetId ? [original.resultAssetId] : []),
        ],
  );
  const next = {
    ...p,
    assets: p.assets.map((a) =>
      obsolete.has(a.id) ? { ...a, discarded: true } : a,
    ),
  };
  assertGenerationReferences(next, task);
  return saveTask(next, task);
}
