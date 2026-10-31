import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { draftItems } from "./items";
import { requestTask, runsOf } from "./requestTask";
import { receiveProductionResult } from "./document";
import { adoptDraft } from "./adoptDraft";
import { addStoryAsset } from "../workspace/storyAssetLibrary";
import { canUseProductionReference } from "../workspace/resourceRoles";
import { applyOperations } from "../assistant/projectCommands";
import { removeNodes } from "../canvas/removeNodes";

const context = (p: ReturnType<typeof fixture>) => ({
  callId: "draft",
  turnId: "turn",
  turn: { projectId: p.id, models: {}, instruction: "Explore" },
});

test("formal shots and their outputs never materialize as scratch cards", () => {
  const p = fixture();
  expect(draftItems(p)).toEqual([]);
  const requested = requestTask(
    p,
    { id: "shot", mediaKind: "image", text: "Frame" },
    context(p),
  );
  const run = runsOf(requested)[0];
  expect(run.position).toBeUndefined();
  const next = receiveProductionResult(requested, asset("new-frame"), {
    ...p.nodes[1],
    canvasGeneration: { task: run, x: 0, y: 0 },
  });
  expect(next.nodes).toHaveLength(p.nodes.length);
  expect(next.nodes[1].shot?.frames?.at(-1)?.assetId).toBe("new-frame");
  expect(draftItems(next)).toEqual([]);
});

test("standalone Agent experiments accept research files without making them shot assets", () => {
  const p = fixture();
  p.assets.push({ ...asset("research"), inLibrary: true, usage: "material" });
  const op = {
    mediaKind: "image",
    text: "Try a composition",
    references: [{ assetId: "research", purpose: "composition" }],
  };
  const requested = requestTask(p, op, context(p));
  const run = runsOf(requested)[0];
  const next = receiveProductionResult(requested, asset("sketch"), {
    ...p.nodes[1],
    canvasGeneration: { task: run, x: 0, y: 0 },
  });
  expect(draftItems(next).some((i) => i.assetId === "sketch")).toBe(true);
  expect(next.nodes[1]).toBe(p.nodes[1]);
  expect(canUseProductionReference(next, next.assets.at(-1)!)).toBe(false);
  expect(() => requestTask(p, { ...op, id: "shot" }, context(p))).toThrow(
    "此资源尚未确认为镜头资产",
  );
});

test("explicit adoption preserves the draft and earlier shot versions; deleting the card keeps the file", () => {
  const p = fixture();
  p.assets.push(asset("sketch"), asset("draft-video", "video"));
  p.nodes.push({
    id: "scratch",
    kind: "asset",
    assetId: "sketch",
    title: "Sketch",
    text: "",
    x: 10,
    y: 20,
  });
  const adopted = adoptDraft(
    adoptDraft(p, "sketch", "shot"),
    "draft-video",
    "shot",
  );
  expect(adopted.nodes[1].shot?.frames?.map((f) => f.assetId)).toEqual([
    "a",
    "b",
    "sketch",
  ]);
  expect(adopted.nodes[1].shot?.takes?.map((f) => f.assetId)).toEqual([
    "v",
    "draft-video",
  ]);
  expect(adopted.nodes[1].resultAssetId).toBeUndefined();
  expect(adoptDraft(adopted, "sketch", "shot")).toBe(adopted);
  const removed = removeNodes(adopted, ["scratch"]);
  expect(removed.assets).toBe(adopted.assets);
  expect(removed.nodes[1].shot).toEqual(adopted.nodes[1].shot);
  expect(
    addStoryAsset(p, "sketch").assets.find((a) => a.id === "sketch")?.usage,
  ).toBe("asset");
});

test("Agent shot edits and task prompt edits remain independent and do not submit generation", () => {
  const p = fixture();
  const requested = requestTask(
    p,
    { id: "shot", mediaKind: "video", text: "Original prompt" },
    context(p),
  );
  const run = runsOf(requested)[0];
  const changed = applyOperations(requested, [
    { op: "update_node", id: "shot", text: "Closer framing" },
    { op: "update_generation", taskKey: run.key, text: "New complete prompt" },
  ]);
  expect(changed.nodes[1].text).toBe("Closer framing");
  expect(changed.nodes[1].shot).toEqual({
    ...p.nodes[1].shot!,
    visualChanged: true,
  });
  expect(runsOf(changed)[0].prompt).toBe("New complete prompt");
  expect(runsOf(changed)[0].status).toBe("AWAITING_CONFIRMATION");
  expect(runsOf(changed)[0].jobId).toBeUndefined();
});
