import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { editTaskPrompt, regenerationDraft } from "./taskEditing";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import type { ProductionTask } from "./types";
const task: ProductionTask = {
  key: "original",
  title: "Pack · 视频",
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
  const next = applyOperations(p, [
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
test("hidden records expose their saved prompt", () => {
  const original = {
    ...task,
    status: "COMPLETED",
    hiddenFromList: true,
    prompt: "新描述",
  };
  const p = saveTask(fixture(), original);
  const result = inspectProject(p, {
    section: "generation",
    taskKey: task.key,
    fields: ["prompt", "hiddenFromList"],
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
    prompt: "新描述",
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
  const next = applyOperations(p, ops, context);
  const run = next.production!.drafts![task.key];
  expect(run).toMatchObject({
    key: task.key,
    prompt: "新描述",
    inputs: task.inputs,
    parameters: task.parameters,
    modelId: "model",
    status: "AWAITING_CONFIRMATION",
    hiddenFromList: false,
  });
  expect(run.jobId).toBeUndefined();
  expect(run.resultAssetIds).toBeUndefined();
  expect(Object.keys(next.production!.drafts!)).toEqual(
    Object.keys(p.production!.drafts!),
  );
  expect(run.turnId).toBe(original.turnId);
  expect(run.createdAt).toBe(original.createdAt);
  expect(applyOperations(next, ops, context)).toEqual(next);
  expect(() => regenerationDraft({ ...task, status: "UNKNOWN" })).toThrow();
});
test("invalid prompts do not change task records", () => {
  const p = saveTask(fixture(), task);
  expect(() => editTaskPrompt(p, task.key, " ")).toThrow();
  expect(() => editTaskPrompt(p, "missing", "x")).toThrow();
  expect(() =>
    applyOperations(p, [
      { op: "update_generation", taskKey: task.key, text: " " },
    ]),
  ).toThrow();
  expect(p.production!.drafts![task.key]).toEqual(task);
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
    const next = applyOperations(p, ops, context);
    expect(Object.keys(next.production!.drafts!)).toEqual([task.key]);
    expect(next.production!.drafts![task.key]).toMatchObject({
      inputs: task.inputs,
      parameters: task.parameters,
      modelId: task.modelId,
      hiddenFromList: false,
      status: "AWAITING_CONFIRMATION",
    });
    expect(applyOperations(next, ops, context)).toEqual(next);
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

test("update_generation replaces or clears references without submitting or changing other task fields", () => {
  const p = saveTask(fixture(), {
    ...task,
    status: "COMPLETED",
    jobId: "job",
    resultAssetIds: ["a"],
  });
  const next = applyOperations(p, [
    {
      op: "update_generation",
      taskKey: task.key,
      references: [{ assetId: "b", role: "first-frame", purpose: "新首帧" }],
    },
  ]);
  const updated = next.production!.drafts![task.key];
  expect(updated.inputs).toMatchObject([
    { assetId: "b", role: "first-frame", purpose: "新首帧" },
  ]);
  expect({ ...updated, inputs: task.inputs }).toEqual(
    p.production!.drafts![task.key],
  );
  const cleared = applyOperations(next, [
    {
      op: "update_generation",
      taskKey: task.key,
      text: "新描述",
      references: [],
    },
  ]);
  expect(cleared.production!.drafts![task.key]).toMatchObject({
    prompt: "新描述",
    inputs: [],
    status: "COMPLETED",
    jobId: "job",
  });
  for (const references of [
    [{ assetId: "missing" }],
    [{ assetId: "b", role: "edit" }],
    "invalid",
  ]) {
    expect(() =>
      applyOperations(p, [
        { op: "update_generation", taskKey: task.key, references },
      ]),
    ).toThrow();
  }
  expect(() =>
    applyOperations(p, [{ op: "update_generation", taskKey: task.key }]),
  ).toThrow();
});
