import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { taskAttachments, taskContext } from "./chat";
import { inspectProject } from "../assistant/inspectProject";
import type { ProductionTask } from "./types";

const task: ProductionTask = {
  key: "existing-task",
  title: "Pack · storyboard",
  turnId: "original-turn",
  createdAt: 1,
  kind: "image",
  mode: "single",
  modelId: "selected-model",
  prompt: "OBSOLETE_GENERATION_PROMPT",
  instruction: "OLD_GENERATION_INSTRUCTION",
  inputs: [],
  parameters: { aspectRatio: "9:16" },
  status: "COMPLETED",
  resultAssetId: "a",
  resultAssetIds: ["a", "b", "missing"],
};

test("chat references send results and settings without old generation text", () => {
  const context = taskContext(task, { image: "selected-model" }, task);
  expect(context).not.toContain(task.prompt);
  expect(context).not.toContain(task.instruction!);
  expect(context).not.toContain("preparedPrompt");
  expect(context).toContain(task.key);
  expect(context).toContain("selected-model");
  const p = fixture();
  const refs = taskAttachments(
    p,
    undefined,
    [{ kind: "asset", id: "a" }],
    task,
  );
  expect(refs.map((r) => r.id)).toEqual(["a", "b"]);
});

test("task metadata omits prompt until explicitly selected for editing or investigation", () => {
  const p = fixture();
  p.production = { drafts: { [task.key]: task } };
  const args = { section: "generation", taskKey: task.key };
  const result = inspectProject(p, args);
  expect(JSON.stringify(result)).not.toContain(task.prompt);
  expect(result.items).toMatchObject([{ id: task.key, resultAssetId: "a" }]);
  expect(
    inspectProject(p, { ...args, fields: ["prompt"] }).items,
  ).toMatchObject([{ id: task.key, prompt: { text: task.prompt } }]);
  const shot = p.nodes[1];
  shot.shot!.takes![0].production = task;
  const media = inspectProject(p, {
    nodeIds: [shot.id],
    fields: ["frames", "takes"],
  });
  expect(JSON.stringify(media)).not.toContain(task.prompt);
  expect(media.details![0].shot!.frames![0].assetId).toBe("a");
  expect(media.details![0].shot!.frames![0]).not.toHaveProperty("prompt");
});
