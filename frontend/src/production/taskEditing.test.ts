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
test("editing every submitted state preserves original request, errors and results", () => {
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
      nextPrompt: "新描述",
    });
  }
});
test("hidden records remain inspectable with the draft and original prompt", () => {
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
      prompt: { text: "旧描述" },
      nextPrompt: { text: "新描述" },
    },
  ]);
});
test("explicit regenerate creates one fresh task with new prompt and same inputs/model/settings", () => {
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
  const run = next.production!.drafts!["retry:call:0"];
  expect(run).toMatchObject({
    sourceTaskKey: task.key,
    prompt: "新描述",
    inputs: task.inputs,
    parameters: task.parameters,
    modelId: "model",
    status: "READY",
    hiddenFromList: false,
  });
  expect(run.jobId).toBeUndefined();
  expect(run.resultAssetIds).toBeUndefined();
  expect(run.nextPrompt).toBeUndefined();
  expect(next.production!.drafts![task.key]).toEqual(original);
  expect(applyOperations(next, next.revision ?? 0, ops, context)).toEqual(next);
  expect(() =>
    regenerationDraft({ ...task, status: "UNKNOWN" }, "no"),
  ).toThrow();
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
