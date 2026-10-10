import { expect, test } from "bun:test";
import { fixture, model } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { createTask } from "./tasks";
import {
  requestTask,
  runsOf,
  type GenerationCommandContext,
} from "./requestTask";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
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

test("a full-reference source can cover fractional editorial timing without changing the shot", () => {
  const { p, context } = setup();
  p.nodes[1].shot!.duration = 5.3;
  const selected = model("minimax/h3/reference-to-video");
  context.turn.task = undefined;
  context.turn.models = { video: selected.id, execution: "automatic" };
  const next = requestTask(
    p,
    {
      op: "request_generation",
      id: "shot",
      mediaKind: "video",
      mode: "multi",
      text: "Complete the packing action by 5.3 seconds, then hold the closed bag.",
      references: [
        {
          assetId: "a",
          role: "reference",
          purpose: "Temporal storyboard: bag states and camera composition",
        },
      ],
      parameters: { duration: 6, aspectRatio: "9:16", resolution: "2K" },
    },
    context,
  );
  const run = runsOf(next)[0];
  expect(run.status).toBe("READY");
  expect(run.targetNodeId).toBe("shot");
  expect(run.modelId).toBe(selected.id);
  expect(next.nodes[1].shot!.duration).toBe(5.3);
  expect(next.nodes[1].shot!.takes).toEqual(p.nodes[1].shot!.takes);
  expect(run.inputs.map((r) => r.role)).toEqual(["reference"]);
  const uploaded = [
    { assetId: "a", kind: "image" as const, url: "https://example.test/board" },
  ];
  const input = inputFor(next, run, selected, uploaded);
  expect(input.duration).toBe(6);
  expect(input.reference_image_urls).toEqual(["https://example.test/board"]);
  expect(input.image_url).toBeUndefined();
  expect(input.end_image_url).toBeUndefined();
  expect(() =>
    inputFor(
      next,
      { ...run, parameters: { duration: 5.3 } },
      selected,
      uploaded,
    ),
  ).not.toThrow();
});
