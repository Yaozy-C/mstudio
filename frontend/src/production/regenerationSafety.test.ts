import { expect, test } from "bun:test";
import { asset, fixture } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { supportsReference } from "./referenceSelection";
import { collectAsset, setAssetUsage } from "../workspace/assetLibrary";
import type { ProductionTask } from "./types";

function setup() {
  const p = fixture();
  p.assets.push({ ...asset("bad"), generated: true });
  const task: ProductionTask = {
    key: "original",
    title: "Pack · storyboard",
    kind: "image",
    mode: "multi",
    modelId: "model",
    prompt: "old",
    inputs: [{ key: "a", assetId: "a", role: "reference", purpose: "product" }],
    parameters: { aspectRatio: "9:16" },
    status: "COMPLETED",
    jobId: "old-job",
    resultAssetId: "bad",
    resultAssetIds: ["bad"],
    turnId: "turn",
    createdAt: 1,
  };
  const project = saveTask(p, task);
  const context = {
    callId: "fix",
    turnId: "fix-turn",
    turn: {
      projectId: project.id,
      models: { execution: "automatic" as const },
      instruction: "修好重新生成",
    },
  };
  return { project, task, context };
}

test("regeneration edits one task and preserves all existing media uses", () => {
  const { project, task, context } = setup();
  project.assets[0].inLibrary = true;
  const ops = [
    {
      op: "regenerate_generation",
      taskKey: task.key,
      text: "corrected",
      references: [{ assetId: "a", purpose: "product" }],
    },
  ];
  const next = applyOperations(project, ops, context);
  expect(Object.keys(next.production!.drafts!)).toEqual([task.key]);
  expect(next.production!.drafts![task.key]).toMatchObject({
    prompt: "corrected",
    status: "AWAITING_CONFIRMATION",
    modelId: task.modelId,
    parameters: task.parameters,
  });
  expect(next.production!.drafts![task.key].resultAssetIds).toBeUndefined();
  expect(next.assets).toEqual(project.assets);
  expect(next.nodes).toEqual(project.nodes);
  expect(next.clips).toEqual(project.clips);
  expect(applyOperations(next, ops, context)).toEqual(next);
});

test("previous drafts remain explicitly reusable; collection and story usage control default discovery", () => {
  const { project, task, context } = setup();
  const next = applyOperations(
    project,
    [
      {
        op: "regenerate_generation",
        taskKey: task.key,
        references: [{ assetId: "bad", role: "edit", purpose: "edit output" }],
      },
    ],
    context,
  );
  expect(next.production!.drafts![task.key].inputs[0].assetId).toBe("bad");
  expect(supportsReference(next.assets.find((a) => a.id === "bad")!)).toBe(
    true,
  );
  const read = (p: typeof project, ids?: string[]) =>
    (
      inspectProject(p, { section: "assets", ids }).items as { id: string }[]
    ).map((a) => a.id);
  expect(read(next)).not.toContain("bad");
  expect(read(next, ["bad"])).toEqual(["bad"]);
  const collected = collectAsset(
    next,
    next.assets.find((a) => a.id === "bad")!,
  );
  expect(read(collected)).not.toContain("bad");
  expect(read(setAssetUsage(collected, "bad", "asset"))).toContain("bad");
});

test("invalid corrections leave completed tasks and media unchanged", () => {
  const { project, task, context } = setup();
  const before = structuredClone(project);
  expect(() =>
    applyOperations(
      project,
      [
        {
          op: "regenerate_generation",
          taskKey: task.key,
          references: [{ assetId: "missing", purpose: "product" }],
        },
      ],
      context,
    ),
  ).toThrow();
  expect(project).toEqual(before);
});
