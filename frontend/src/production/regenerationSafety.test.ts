import { expect, test } from "bun:test";
import { asset, fixture, model } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { requestTask } from "./requestTask";
import { inputFor } from "./request";
import { supportsReference } from "./referenceSelection";
import { setFrameInput } from "./frameInputs";
import { collectAsset } from "../workspace/assetLibrary";
import { receipt } from "../domain/receipt";
import type { ProductionTask } from "./types";

function setup() {
  const p = fixture();
  p.assets.push({ ...asset("bad"), generated: true });
  const task: ProductionTask = {
    key: "original",
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

test("correction replaces prompt and references on one task and revokes old results", () => {
  const { project, task, context } = setup();
  const ops = [
    {
      op: "regenerate_generation",
      taskKey: task.key,
      text: "corrected",
      references: ["a", "b", "reference"].map((assetId) => ({
        assetId,
        purpose: "product",
      })),
    },
  ];
  const next = applyOperations(project, ops, context);
  expect(Object.keys(next.production!.drafts!)).toEqual([task.key]);
  expect(next.production!.drafts![task.key]).toMatchObject({
    key: task.key,
    prompt: "corrected",
    status: "READY",
    modelId: task.modelId,
    parameters: task.parameters,
  });
  expect(
    next.production!.drafts![task.key].inputs.map((i) => i.assetId),
  ).toEqual(["a", "b", "reference"]);
  expect(next.production!.drafts![task.key].resultAssetIds).toBeUndefined();
  expect(next.assets.find((a) => a.id === "bad")?.discarded).toBe(true);
  expect(project.assets.find((a) => a.id === "bad")?.discarded).toBeUndefined();
  expect(applyOperations(next, ops, context)).toEqual(next);
  expect(receipt(project, next, ops, "fix").discardedAssetIds).toEqual(["bad"]);
});

test("discarded outputs cannot be selected, recollected into usability, or submitted to video", () => {
  const { project, task, context } = setup();
  const next = applyOperations(
    project,
    [{ op: "regenerate_generation", taskKey: task.key }],
    context,
  );
  const bad = next.assets.find((a) => a.id === "bad")!;
  expect(supportsReference(bad)).toBe(false);
  expect(
    supportsReference(
      collectAsset(next, bad).assets.find((a) => a.id === bad.id)!,
    ),
  ).toBe(false);
  expect(
    (
      inspectProject(next, { section: "assets" }).items as { id: string }[]
    ).some((a) => a.id === bad.id),
  ).toBe(false);
  for (const kind of ["image", "video"] as const) {
    expect(() =>
      requestTask(
        next,
        {
          mediaKind: kind,
          text: "next",
          references: [{ assetId: "bad", purpose: "product" }],
        },
        { ...context, callId: kind },
      ),
    ).toThrow("已废弃");
  }
  const video: ProductionTask = {
    ...task,
    kind: "video",
    mode: "single",
    prompt: "animate",
    inputs: [
      { key: "bad", assetId: "bad", role: "first-frame", purpose: "product" },
    ],
  };
  expect(() =>
    setFrameInput(video, next, "first-frame", video.inputs[0]),
  ).toThrow("可用");
  // A previously saved video draft must also fail at the actual submission boundary.
  expect(() =>
    inputFor(next, video, model("minimax/h3/image-to-video")),
  ).toThrow("已废弃");
  expect(() =>
    requestTask(
      next,
      { mediaKind: "video", text: "next" },
      {
        ...context,
        callId: "inherited",
        turn: { ...context.turn, task: video },
      },
    ),
  ).toThrow("已废弃");
});

test("invalid corrections are atomic and result reuse requires an explicit keep flag", () => {
  const { project, task, context } = setup();
  const op = {
    op: "regenerate_generation",
    taskKey: task.key,
    references: [{ assetId: "bad", role: "edit", purpose: "edit output" }],
  };
  expect(() => applyOperations(project, [op], context)).toThrow("已废弃");
  expect(project.production!.drafts![task.key].status).toBe("COMPLETED");
  expect(project.assets.find((a) => a.id === "bad")?.discarded).toBeUndefined();
  const next = applyOperations(
    project,
    [{ ...op, keepPreviousResults: true }],
    context,
  );
  expect(next.assets.find((a) => a.id === "bad")?.discarded).toBeUndefined();
  expect(next.production!.drafts![task.key].inputs[0].assetId).toBe("bad");
  expect(() =>
    applyOperations(
      project,
      [{ ...op, references: [{ assetId: "v", role: "first-frame" }] }],
      context,
    ),
  ).toThrow("用途");
  const cleared = applyOperations(
    project,
    [{ ...op, references: [] }],
    context,
  );
  expect(cleared.production!.drafts![task.key].inputs).toEqual([]);
});
