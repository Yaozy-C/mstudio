import { expect, test } from "bun:test";
import { fixture, asset, model } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { createTask } from "./tasks";
import {
  requestTask,
  runsOf,
  type GenerationCommandContext,
} from "./requestTask";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { receiveProductionResult, jobStatus } from "./document";
import { restoreProject } from "../workspace/restoreProject";
import { canvasSnapshot, inputFor } from "./request";

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
test("image tasks reject video modes instead of silently discarding them", () => {
  const { p, context, op } = setup();
  expect(() => requestTask(p, { ...op, mode: "single" }, context)).toThrow();
  const run = runsOf(requestTask(p, op, context))[0];
  expect(run.kind).toBe("image");
  expect(run.mode).toBe("multi");
});
test("Agent creates a durable, scoped conversation task with a human-selected model", () => {
  const { p, context, op } = setup();
  const next = applyOperations(p, [op], context);
  const run = runsOf(next)[0];
  expect(run.status).toBe("AWAITING_CONFIRMATION");
  expect(run.modelId).toBe("chosen-image");
  expect(run.inputs.map((r) => r.assetId)).toEqual(["a"]);
  expect(run.inputs[0].role).toBe("edit");
  expect(next.nodes).toBe(p.nodes);
  expect(
    (inspectProject(next, { section: "generation" }) as { items: unknown[] })
      .items,
  ).toHaveLength(1);
  expect(() => applyOperations(p, [op])).toThrow("上下文");
  expect(() =>
    requestTask(p, { ...op, mediaModelId: "unselected" }, context),
  ).toThrow("用户选择");
  expect(() =>
    requestTask(next, op, {
      ...context,
      callId: "new",
      turn: { ...context.turn, projectId: "other" },
    }),
  ).toThrow("上下文");
});
test("automatic execution requires explicit mode and a selected model", () => {
  const { p, context, op } = setup();
  context.turn.models.execution = "automatic";
  const next = requestTask(p, op, context);
  expect(runsOf(next)[0].status).toBe("READY");
  context.turn.models.image = "";
  expect(runsOf(requestTask(p, op, context))[0].status).toBe(
    "AWAITING_CONFIRMATION",
  );
});
test("a tool retry cannot duplicate the same paid request, while subsequent user turns can iterate", () => {
  const { p, context, op } = setup();
  const first = requestTask(p, op, context);
  expect(requestTask(first, op, context)).toBe(first);
  expect(
    runsOf(
      requestTask(
        first,
        { ...op, parameters: { duration: 7 } },
        { ...context, callId: "distinct" },
      ),
    ),
  ).toHaveLength(2);
  expect(
    runsOf(
      requestTask(first, op, {
        ...context,
        callId: "next",
        turnId: "next-turn",
      }),
    ),
  ).toHaveLength(2);
  expect(() =>
    applyOperations(p, [op, { op: "unsupported_operation" }], context),
  ).toThrow();
  expect(runsOf(p)).toHaveLength(0);
});
test("first and last frame requests preserve exact references without a frame confirmation step", () => {
  let { p, context, op } = setup();
  const references = [
    { assetId: "a", role: "first-frame" },
    { assetId: "b", role: "last-frame" },
  ];
  const run = runsOf(
    requestTask(
      p,
      { ...op, mediaKind: "video", mode: "ends", references },
      context,
    ),
  )[0];
  expect(run.modelId).toBe("chosen-video");
  expect(canvasSnapshot(p, run, { x: 0, y: 0 }).task).toEqual(run);
  const input = inputFor(p, run, model("minimax/h3/image-to-video"), [
    { assetId: "a", kind: "image", url: "https://example.test/a" },
    { assetId: "b", kind: "image", url: "https://example.test/b" },
  ]);
  expect(input.image_url).toBe("https://example.test/a");
  expect(input.end_image_url).toBe("https://example.test/b");
  expect(() =>
    requestTask(
      p,
      { ...op, references: [{ assetId: "a", role: "video-reference" }] },
      context,
    ),
  ).toThrow("用途");
  expect(() =>
    requestTask(p, { ...op, references: [{ assetId: "missing" }] }, context),
  ).toThrow("已有");
});

test("results belong to their conversation task; save, undo, and later iterations retain actual job state", () => {
  const { p, context, op } = setup();
  let next = requestTask(p, op, context);
  const run = { ...runsOf(next)[0], jobId: "job-1" };
  next.production!.drafts![run.key] = run;
  const source = {
    ...p.nodes[1],
    canvasGeneration: canvasSnapshot(next, run, { x: 500, y: 100 }),
  };
  next = receiveProductionResult(next, asset("generated-image"), source);
  next = jobStatus(next, "job-1", "COMPLETED");
  expect(runsOf(next)[0].resultAssetId).toBe("generated-image");
  expect(receiveProductionResult(next, asset("generated-image"), source)).toBe(
    next,
  );
  const restored = restoreProject(p, JSON.parse(JSON.stringify(next)));
  expect(runsOf(restored)[0].status).toBe("COMPLETED");
  expect(restored.assets.some((a) => a.id === "generated-image")).toBe(true);
});

test("composer parameters survive assigning a new shot after preflight", () => {
  const { p, context, op } = setup();
  context.turn.task = {
    ...createTask(p, [], "image"),
    parameters: { aspectRatio: "9:16", resolution: "2K" },
  };
  context.turn.models.execution = "automatic";
  const next = requestTask(
    p,
    { ...op, id: "shot", canvasTaskKey: context.turn.task.key },
    context,
  );
  const run = runsOf(next)[0];
  expect(run.parameters).toEqual({ aspectRatio: "9:16", resolution: "2K" });
  expect(run.status).toBe("READY");
});

test("a shot task does not implicitly add its images or own new generation results", () => {
  const { p, context, op } = setup();
  context.turn.task = createTask(p, [], "video");
  const next = requestTask(
    p,
    {
      ...op,
      id: "shot",
      canvasTaskKey: context.turn.task.key,
      mediaKind: "video",
      mode: "multi",
    },
    context,
  );
  const run = runsOf(next)[0];
  expect(run.inputs).toEqual([]);
  expect(run.ownerId).toBeUndefined();
  const done = receiveProductionResult(next, asset("independent", "video"), {
    ...p.nodes[1],
    canvasGeneration: canvasSnapshot(next, run, { x: 3000, y: 100 }),
  });
  expect(done.nodes[1]).toEqual(p.nodes[1]);
  expect(done.nodes.some((n) => n.assetId === "independent")).toBe(true);
});

test("100 generation tasks persist across tool batches without replaying the same call", () => {
  const { p, context, op } = setup();
  let next = p;
  for (let i = 0; i < 100; i++) {
    next = requestTask(
      next,
      { ...op, text: `Shot ${i}: move closer` },
      { ...context, callId: `batch-${Math.floor(i / 10)}` },
      i % 10,
    );
  }
  const restored = JSON.parse(JSON.stringify(next));
  expect(runsOf(restored)).toHaveLength(100);
  expect(
    requestTask(
      restored,
      { ...op, text: "Shot 99: move closer" },
      { ...context, callId: "batch-9" },
      9,
    ),
  ).toBe(restored);
  expect(
    runsOf(
      requestTask(
        restored,
        { ...op, text: "Shot 99: move closer" },
        { ...context, callId: "replayed" },
      ),
    ),
  ).toHaveLength(101);
});

test("a catalog model selected in conversation creates a task without a dropdown selection", () => {
  const { p, context, op } = setup();
  context.turn.models.image = "";
  const requested = { ...op, mediaModelId: "catalog-selected-image" };
  const run = runsOf(requestTask(p, requested, context))[0];
  expect(run.modelId).toBe("catalog-selected-image");
  expect(run.status).toBe("AWAITING_CONFIRMATION");
  context.turn.models.execution = "automatic";
  expect(runsOf(requestTask(p, requested, context))[0].status).toBe("READY");
  expect(() => requestTask(p, { ...op, mediaModelId: 42 }, context)).toThrow(
    "模型 ID",
  );
});

test("advertised prompt paths save the shot draft and generation prompt separately", () => {
  const { p, context } = setup();
  const shot = p.nodes.find((n) => n.shot)!;
  const next = applyOperations(
    p,
    [
      { op: "update_node", id: shot.id, shot: { prompt: "Saved video draft" } },
      {
        op: "request_generation",
        id: shot.id,
        mediaKind: "video",
        text: "Actual generation prompt",
        mode: "multi",
        references: [
          { assetId: "a", role: "reference", purpose: "Product identity" },
        ],
        parameters: { duration: 5, resolution: "768P" },
      },
    ],
    { ...context, turn: { ...context.turn, task: undefined } },
  );
  expect(next.nodes.find((n) => n.id === shot.id)!.shot!.prompt).toBe(
    "Saved video draft",
  );
  const task = runsOf(next)[0];
  expect(task.prompt).toBe("Actual generation prompt");
  expect(task.mode).toBe("multi");
  expect(task.inputs[0].role).toBe("reference");
  const updated = applyOperations(next, [
    { op: "update_generation", taskKey: task.key, text: "Revised task prompt" },
  ]);
  expect(updated.production!.drafts![task.key].prompt).toBe(
    "Revised task prompt",
  );
  expect(updated.nodes.find((n) => n.id === shot.id)!.shot!.prompt).toBe(
    "Saved video draft",
  );
});
