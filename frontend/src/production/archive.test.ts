import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { createTask } from "./tasks";
import { archiveTask } from "./archive";
import { inspectProject } from "../assistant/inspectProject";
import { changeBatch } from "./batch";
import { applyOperations } from "../assistant/projectCommands";
import { receipt } from "../domain/receipt";
import { prepareGeneration } from "../domain/generation";
import { model } from "./fixtures.test-helper";

test("archived tasks expose facts, retain full records for exact reads and cannot resume in a batch", () => {
  let p = fixture();
  const task = {
    ...createTask(p, [], "image"),
    key: "run:archived",
    title: "Pack · storyboard",
    turnId: "turn",
    createdAt: 1,
    status: "AWAITING_CONFIRMATION" as const,
    modelId: "m",
    prompt: "private old prompt",
    parameters: { width: 1024 },
    inputs: [
      {
        key: "a",
        assetId: "a",
        role: "reference" as const,
        purpose: "private input purpose",
      },
    ],
  };
  p = archiveTask(saveTask(p, task), task.key, true);
  const list = inspectProject(p, {
    section: "generation",
    fields: ["prompt", "inputs", "parameters"],
  });
  expect(JSON.stringify(list)).not.toContain("private");
  expect(JSON.stringify(list)).not.toContain("1024");
  expect(JSON.stringify(list)).toContain('"archived":true');
  expect(
    JSON.stringify(
      inspectProject(p, {
        section: "generation",
        taskKey: task.key,
        fields: ["prompt", "inputs"],
      }),
    ),
  ).toContain("private old prompt");
  expect(
    changeBatch(p, "turn", "resume").production!.drafts![task.key].status,
  ).toBe("AWAITING_CONFIRMATION");
  const restored = archiveTask(p, task.key, false);
  expect(
    JSON.stringify(
      inspectProject(restored, { section: "generation", fields: ["prompt"] }),
    ),
  ).toContain("private old prompt");
  expect(p.production!.drafts![task.key].prompt).toBe(task.prompt);
  expect(() =>
    archiveTask(
      saveTask(p, { ...task, status: "IN_PROGRESS" }),
      task.key,
      true,
    ),
  ).toThrow();
});

test("Agent archives obsolete pending tasks with a finished receipt and can restore them", () => {
  const task = {
    ...createTask(fixture(), [], "video"),
    key: "obsolete",
    title: "Pack · video",
    status: "AWAITING_CONFIRMATION",
    modelId: "removed-model",
    parameters: { duration: 5 },
    prompt: "Original prompt",
  };
  const before = saveTask(fixture(), task);
  const operations = [
    { op: "archive_generation", taskKey: task.key, archived: true },
  ];
  const after = applyOperations(before, operations);
  expect(after.production!.drafts!.obsolete).toEqual({
    ...task,
    hiddenFromList: true,
  });
  expect(before.production!.drafts!.obsolete.hiddenFromList).toBeUndefined();
  expect(after.assets).toEqual(before.assets);
  expect(after.nodes).toEqual(before.nodes);
  const saved = receipt(before, after, operations, "archive");
  expect(saved.generationTasks).toEqual([]);
  expect(saved.updatedTasks[0]).toMatchObject({
    id: task.key,
    archived: true,
    continuation: { state: "finished", action: "read_details_on_request" },
  });
  expect(saved.updatedTasks[0]).not.toHaveProperty("prompt");
  expect(() =>
    prepareGeneration(after, task.key, model("minimax/h3/reference-to-video")),
  ).toThrow("归档");
  expect(
    applyOperations(after, [{ ...operations[0], archived: false }]).production!
      .drafts!.obsolete,
  ).toEqual({ ...task, hiddenFromList: false });
  expect(() =>
    applyOperations(before, [{ ...operations[0], taskKey: "missing" }]),
  ).toThrow("不存在");
});

test("Agent archive batch rejects active and uncertain tasks without partial edits", () => {
  for (const status of [
    "READY",
    "UPLOADING",
    "SUBMITTING",
    "IN_QUEUE",
    "IN_PROGRESS",
    "RECEIVING",
    "UNKNOWN",
    "CANCEL_REQUESTED",
  ]) {
    let before = saveTask(fixture(), {
      ...createTask(fixture(), [], "video"),
      key: "old",
      status: "AWAITING_CONFIRMATION",
    });
    before = saveTask(before, {
      ...createTask(before, [], "video"),
      key: "busy",
      status,
    });
    const snapshot = structuredClone(before);
    expect(() =>
      applyOperations(before, [
        { op: "archive_generation", taskKey: "old", archived: true },
        { op: "archive_generation", taskKey: "busy", archived: true },
      ]),
    ).toThrow("停止");
    expect(before).toEqual(snapshot);
  }
});
