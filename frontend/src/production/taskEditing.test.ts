import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { editTaskPrompt, regenerationDraft } from "./taskEditing";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import type { ProductionTask } from "./types";
const task: ProductionTask = {
  key: "original",
  turnId: "turn",
  createdAt: 1,
  kind: "video",
  mode: "single",
  modelId: "model",
  prompt: "旧描述",
  inputs: [{ key: "a", assetId: "a", role: "first-frame", purpose: "产品" }],
  parameters: { resolution: "480P", duration: 5 },
  status: "AWAITING_CONFIRMATION",
};
test("editing an unsubmitted task changes its prompt without submitting", () => {
  const p = saveTask(fixture(), task);
  const next = applyOperations(p, p.revision ?? 0, [
    { op: "update_generation", taskKey: task.key, text: "掀盖" },
  ]);
  expect(next.production!.drafts![task.key]).toEqual({
    ...task,
    prompt: "掀盖",
    error: undefined,
  });
  expect(p.production!.drafts![task.key].prompt).toBe("旧描述");
  expect(Object.keys(next.production!.drafts!)).toHaveLength(1);
});
test("editing every submitted state updates the single prompt without changing job or results", () => {
  for (const status of [
    "READY",
    "UPLOADING",
    "SUBMITTING",
    "IN_QUEUE",
    "IN_PROGRESS",
    "UNKNOWN",
    "RECEIVING",
    "COMPLETED",
    "FAILED",
    "CANCELLED",
  ]) {
    const original = {
      ...task,
      status,
      jobId: "job",
      requestId: "remote",
      error: "failure",
      resultAssetIds: ["result"],
    };
    const next = editTaskPrompt(
      saveTask(fixture(), original),
      task.key,
      "新描述",
    );
    expect(next.production!.drafts![task.key]).toEqual({
      ...original,
      prompt: "新描述",
    });
  }
});
test("hidden records expose one prompt after migrating a saved edit", () => {
  const original = {
    ...task,
    status: "COMPLETED",
    hiddenFromList: true,
    nextPrompt: "新描述",
  };
  const p = saveTask(fixture(), original);
  const result = inspectProject(p, {
    section: "generation",
    taskKey: task.key,
  });
  expect(result.items).toMatchObject([
    {
      id: "original",
      hiddenFromList: true,
      prompt: { text: "新描述" },
    },
  ]);
});
test("explicit regenerate resets the existing task once with new prompt and the same settings", () => {
  const original = {
    ...task,
    status: "COMPLETED",
    hiddenFromList: true,
    jobId: "old-job",
    resultAssetIds: ["result"],
    nextPrompt: "新描述",
  };
  const p = saveTask(fixture(), original);
  const context = {
    callId: "call",
    turnId: "new-turn",
    turn: {
      projectId: p.id,
      models: { execution: "automatic" as const },
      instruction: "改好后重新生成",
    },
  };
  const ops = [{ op: "regenerate_generation", taskKey: task.key }];
  const next = applyOperations(p, p.revision ?? 0, ops, context);
  const run = next.production!.drafts![task.key];
  expect(run).toMatchObject({
    key: task.key,
    prompt: "新描述",
    inputs: task.inputs,
    parameters: task.parameters,
    modelId: "model",
    status: "READY",
    hiddenFromList: false,
  });
  expect(run.jobId).toBeUndefined();
  expect(run.resultAssetIds).toBeUndefined();
  expect("nextPrompt" in run).toBe(false);
  expect(Object.keys(next.production!.drafts!)).toEqual(
    Object.keys(p.production!.drafts!),
  );
  expect(run.turnId).toBe(original.turnId);
  expect(run.createdAt).toBe(original.createdAt);
  expect(applyOperations(next, next.revision ?? 0, ops, context)).toEqual(next);
  expect(() => regenerationDraft({ ...task, status: "UNKNOWN" })).toThrow();
});
test("invalid prompt and stale revisions do not change task records", () => {
  const p = saveTask(fixture(), task);
  expect(() => editTaskPrompt(p, task.key, " ")).toThrow();
  expect(() => editTaskPrompt(p, "missing", "x")).toThrow();
  expect(() =>
    applyOperations(p, -1, [
      { op: "update_generation", taskKey: task.key, text: "x" },
    ]),
  ).toThrow();
  expect(p.production!.drafts![task.key]).toEqual(task);
});
test("legacy prompt edits migrate once without losing text or touching job metadata", () => {
  const legacy = {
    ...task,
    status: "COMPLETED",
    prompt: "submitted",
    nextPrompt: "edited",
    jobId: "immutable-job",
    resultAssetIds: ["image"],
  };
  const p = saveTask(fixture(), legacy);
  const restored = p.production!.drafts!.original;
  expect(restored.prompt).toBe("edited");
  expect("nextPrompt" in restored).toBe(false);
  expect(restored.jobId).toBe("immutable-job");
  expect(restored.resultAssetIds).toEqual(["image"]);
  expect(regenerationDraft(restored).prompt).toBe("edited");
  expect(legacy.prompt).toBe("submitted");
});

test("explicit generation reuses an unsubmitted hidden task and respects execution mode", () => {
  for (const execution of ["automatic", "confirm"] as const) {
    const p = saveTask(fixture(), { ...task, hiddenFromList: true });
    const context = {
      callId: "resume",
      turnId: "resume-turn",
      turn: { projectId: p.id, models: { execution }, instruction: "重新生成" },
    };
    const ops = [{ op: "regenerate_generation", taskKey: task.key }];
    const next = applyOperations(p, p.revision ?? 0, ops, context);
    expect(Object.keys(next.production!.drafts!)).toEqual([task.key]);
    expect(next.production!.drafts![task.key]).toMatchObject({
      inputs: task.inputs,
      parameters: task.parameters,
      modelId: task.modelId,
      hiddenFromList: false,
      status: execution === "automatic" ? "READY" : "AWAITING_CONFIRMATION",
    });
    expect(applyOperations(next, next.revision ?? 0, ops, context)).toEqual(
      next,
    );
  }
});

test("regeneration still blocks pending and uncertain submissions", () => {
  for (const status of [
    "READY",
    "UPLOADING",
    "SUBMITTING",
    "IN_QUEUE",
    "IN_PROGRESS",
    "RECEIVING",
    "CANCEL_REQUESTED",
    "UNKNOWN",
  ]) {
    expect(() => regenerationDraft({ ...task, status })).toThrow();
  }
  expect(() => regenerationDraft({ ...task, jobId: "existing-job" })).toThrow();
  expect(() =>
    regenerationDraft({ ...task, submissionId: "existing-submission" }),
  ).toThrow();
});
