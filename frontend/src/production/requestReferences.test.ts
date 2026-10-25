import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { createTask } from "./tasks";
import {
  requestTask,
  runsOf,
  type GenerationCommandContext,
} from "./requestTask";
import { applyOperations } from "../assistant/projectCommands";
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
    title: "Pack · 分镜图",
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
    title: "Character reference",
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

test("asset generation delivers a standalone candidate canvas card", () => {
  const { p, context } = setup();
  let next = requestTask(
    p,
    {
      mediaKind: "image",
      generationPurpose: "asset",
      title: "Character reference",
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
  expect(next.assets.find((a) => a.id === image.id)?.inLibrary).toBeUndefined();
  expect(next.nodes.filter((n) => n.assetId === image.id)).toHaveLength(1);
  expect(
    productionItems(next).find((n) => n.assetId === image.id),
  ).toMatchObject({
    kind: "reference",
    ...run.position,
  });
  expect(next.nodes.find((n) => n.id === "shot")).toEqual(
    p.nodes.find((n) => n.id === "shot"),
  );
  next = applyOperations(next, [
    {
      op: "set_references",
      id: "shot",
      referenceMode: "upsert",
      references: [{ assetId: image.id, purpose: "character and wardrobe" }],
    },
  ]);
  expect(next.nodes.find((n) => n.id === "shot")!.references).toHaveLength(2);
  next = requestTask(
    next,
    {
      mediaKind: "image",
      text: "Same person on steps",
      title: "Character on steps",
      references: [{ assetId: image.id, purpose: "character and wardrobe" }],
    },
    { ...context, callId: "downstream" },
  );
  expect(
    runsOf(next).find((t) => t.key.includes("downstream"))!.inputs[0].assetId,
  ).toBe(image.id);
});

test("shot references do not silently become task inputs", () => {
  const { p, context } = setup();
  const shot = p.nodes.find((n) => n.shot)!;
  shot.references = [{ assetId: "a", purpose: "product identity" }];
  const op = {
    id: shot.id,
    title: "Pack · 分镜图",
    mediaKind: "image",
    text: "Make shot",
  };
  const explicitContext = {
    ...context,
    turn: { ...context.turn, task: undefined },
  };
  const implicit = requestTask(p, op, explicitContext);
  expect(runsOf(implicit)[0].inputs).toEqual([]);
  const explicit = requestTask(
    p,
    { ...op, references: shot.references },
    explicitContext,
  );
  expect(runsOf(explicit)[0].inputs.map((r) => r.assetId)).toEqual(["a"]);
  expect(implicit.nodes).toBe(p.nodes);
});
