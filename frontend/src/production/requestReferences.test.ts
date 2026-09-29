import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { createTask } from "./tasks";
import {
  requestTask,
  runsOf,
  type GenerationCommandContext,
} from "./requestTask";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { receiveProductionResult } from "./document";
function setup() {
  const p = fixture();
  const task = createTask(
    p,
    productionItems(p).filter((n) => n.assetId === "a"),
  );
  const context: GenerationCommandContext = {
    callId: "call",
    turnId: "turn",
    turn: {
      projectId: p.id,
      task,
      models: { image: "chosen-image", video: "chosen-video" },
      instruction: "保持包不变，拉近画面。",
    },
  };
  const op = {
    op: "request_generation",
    mediaKind: "image",
    text: "Keep the bag. Closer shot.",
    canvasTaskKey: task.key,
  };
  return { p, task, context, op };
}
test("reference image tasks are standalone and require explicit inputs even with a selected shot", () => {
  const { p, context, task } = setup();
  const op = {
    op: "request_generation",
    mediaKind: "image",
    generationPurpose: "asset",
    text: "Full character on pure white",
    references: [],
  };
  context.turn.task = {
    ...task,
    ownerId: "selected-shot",
    parameters: task.parameters,
  } as typeof task;
  const next = requestTask(p, op, context);
  const run = runsOf(next)[0];
  expect(run.generationPurpose).toBe("asset");
  expect(run.ownerId).toBeUndefined();
  expect(run.targetNodeId).toBeUndefined();
  expect(run.parameters).toBeUndefined();
  expect(run.inputs).toEqual([]);
  expect(run.modelId).toBe("chosen-image");
  expect(next.nodes).toBe(p.nodes);
  for (const bad of [
    { ...op, mediaKind: "video" },
    { ...op, id: "selected-shot" },
    { ...op, references: undefined },
    { ...op, canvasTaskKey: task.key },
  ])
    expect(() => requestTask(p, bad, context)).toThrow();
});

test("asset generation delivers a reusable library image that can be passed directly to a shot", () => {
  const { p, context } = setup();
  let next = requestTask(
    p,
    {
      mediaKind: "image",
      generationPurpose: "asset",
      text: "Subject on pure white",
      references: [],
    },
    context,
  );
  const run = runsOf(next)[0];
  const image = asset("shared-person", "image");
  next = receiveProductionResult(next, image, {
    id: "source",
    kind: "asset",
    title: "Asset",
    text: "",
    x: 0,
    y: 0,
    canvasGeneration: { task: run, x: 30, y: 40 },
  });
  expect(next.assets.find((a) => a.id === image.id)?.inLibrary).toBe(true);
  const record = next.nodes.find((n) => n.assetId === image.id)!;
  expect(record.kind).toBe("asset");
  expect(record.shot).toBeUndefined();
  next = applyOperations(next, next.revision ?? 0, [
    {
      op: "update_node",
      id: record.id,
      title: "Character reference",
      text: "ready; white background inspected; outfit fixed",
    },
  ]);
  expect(
    inspectProject(next, { nodeIds: [record.id], fields: ["text", "assetId"] })
      .details?.[0].assetId,
  ).toBe(image.id);
  next = requestTask(
    next,
    {
      mediaKind: "image",
      text: "Same person on steps",
      references: [{ assetId: image.id, purpose: "character and wardrobe" }],
    },
    { ...context, callId: "downstream" },
  );
  expect(
    runsOf(next).find((t) => t.key.includes("downstream"))!.inputs[0].assetId,
  ).toBe(image.id);
  expect(() =>
    applyOperations(next, next.revision ?? 0, [
      { op: "update_node", id: record.id, assetId: "missing" },
    ]),
  ).toThrow();
});

test("shot image references feed new tasks without changing historical inputs", () => {
  const { p, context } = setup();
  const shot = p.nodes.find((n) => n.shot)!;
  shot.references = [{ assetId: "a", purpose: "product identity" }];
  const op = { id: shot.id, mediaKind: "image", text: "Make shot" };
  const first = requestTask(p, op, {
    ...context,
    turn: { ...context.turn, task: undefined },
  });
  const old = structuredClone(runsOf(first)[0]);
  expect(old.inputs.map((i) => i.assetId)).toEqual(["a"]);
  const changed = {
    ...first,
    nodes: first.nodes.map((n) =>
      n.id === shot.id ? { ...n, references: [] } : n,
    ),
  };
  const next = requestTask(changed, op, {
    ...context,
    callId: "new",
    turn: { ...context.turn, task: undefined },
  });
  expect(next.production!.drafts![old.key]).toEqual(old);
  expect(runsOf(next).find((r) => r.key === "run:new:0")!.inputs).toEqual([]);
  const explicit = requestTask(
    first,
    { ...op, references: [] },
    {
      ...context,
      callId: "explicit",
      turn: { ...context.turn, task: undefined },
    },
  );
  expect(
    runsOf(explicit).find((r) => r.key === "run:explicit:0")!.inputs,
  ).toEqual([]);
});
