import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { createTask } from "./tasks";
import { archiveTask } from "./archive";
import { inspectProject } from "../assistant/inspectProject";
import { changeBatch } from "./batch";
import { describeAttachment } from "../assistant/attachments";
import { applyOperations } from "../assistant/projectCommands";

test("archived tasks expose facts, retain full records for exact reads and cannot resume in a batch", () => {
  let p = fixture();
  const task = {
    ...createTask(p, [], "image"),
    key: "run:archived",
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

test("discarded results cannot reenter Agent attachments, canvas or generation references", () => {
  const p = fixture();
  p.assets[0].discarded = true;
  expect(
    describeAttachment(p, { kind: "asset", id: p.assets[0].id }),
  ).toBeNull();
  expect(() =>
    applyOperations(p, [
      {
        op: "add_node",
        id: "reused",
        kind: "asset",
        assetId: p.assets[0].id,
        title: "bad",
      },
    ]),
  ).toThrow("废弃");
  expect(() =>
    applyOperations(p, [
      {
        op: "request_generation",
        text: "x",
        references: [{ assetId: p.assets[0].id, role: "reference" }],
      },
    ]),
  ).toThrow("废弃");
});
