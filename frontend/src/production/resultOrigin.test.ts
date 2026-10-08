import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { resultOrigin } from "./resultOrigin";
import { resultPlacement } from "./resultPlacement";
import { receiveProductionResult } from "./document";
import { productionItems } from "./items";
import { attachmentInput } from "./attachmentInput";
import { createTask } from "./tasks";
import { directTask } from "./directTask";
import { model } from "./fixtures.test-helper";
import { selectMediaModel } from "./frameInputs";
import type { ProductionTask } from "./types";
const task = (): ProductionTask => ({
  key: "run:direct:edit",
  turnId: "direct:edit",
  createdAt: 1,
  kind: "image",
  mode: "multi",
  modelId: "image",
  prompt: "修正上盖",
  status: "COMPLETED",
  inputs: [
    { key: "asset:a", assetId: "a", role: "reference", purpose: "参考图" },
  ],
});
test("ordinary references neither infer ownership nor select a placement anchor", () => {
  const p = fixture();
  expect(resultOrigin(p, task())).toEqual({
    ownerId: undefined,
    origin: undefined,
  });
  expect(
    createTask(
      p,
      productionItems(p).filter((n) => n.assetId === "a"),
    ).inputs,
  ).toHaveLength(1);
  expect(attachmentInput(p, { kind: "node", id: "shot" })?.assetId).toBe("");
  expect(attachmentInput(p, { kind: "node", id: "shot" })?.role).toBe("script");
});
test("explicit edits create an independent card beside the source and preserve the original", () => {
  let p = fixture();
  const origin = productionItems(p).find((n) => n.assetId === "a")!;
  p.production = { positions: { [origin.key]: { x: 900, y: 900 } } };
  const t = task();
  t.inputs[0].role = "edit";
  t.position = resultPlacement(p, t);
  expect(t.position).toEqual({ x: 1182, y: 900 });
  const source = {
    ...p.nodes[1],
    canvasGeneration: { task: t, ...t.position },
  };
  const next = receiveProductionResult(p, asset("result"), source);
  const result = productionItems(next).find((n) => n.assetId === "result")!;
  expect(result.ownerId).toBeUndefined();
  expect(result.x).toBe(1182);
  expect(result.y).toBe(900);
  expect(next.nodes[1]).toEqual(p.nodes[1]);
  expect(receiveProductionResult(next, asset("result"), source)).toBe(next);
});
test("unassigned generations find free visible space and avoid overlaps", () => {
  const p = fixture();
  p.production = { viewport: { x: -4000, y: -1000, scale: 1 } };
  const t = task();
  t.inputs = [];
  const pos = resultPlacement(p, t);
  expect(pos.x).toBe(4032);
  expect(pos.y).toBe(1080);
  p.nodes.push({
    id: "occupied",
    kind: "asset",
    title: "",
    text: "",
    x: 0,
    y: 0,
  });
  p.production.positions = { "node:occupied": pos };
  expect(resultPlacement(p, t)).not.toEqual(pos);
});
test("model changes retain unsupported roles and independent images work as video inputs", () => {
  const p = fixture();
  p.nodes.push({
    id: "standalone",
    kind: "asset",
    assetId: "a",
    title: "",
    text: "",
    x: 0,
    y: 0,
  });
  const item = productionItems(p).find((n) => n.key === "node:standalone")!;
  const draft = createTask(p, [item], "video");
  draft.prompt = "推进";
  expect(draft.inputs.map((r) => r.assetId)).toEqual(["a"]);
  expect(draft.inputs[0].role).toBe("first-frame");
  const run = directTask(p, draft, model("minimax/h3/image-to-video"), "v");
  expect(run.ownerId).toBeUndefined();
  expect(
    selectMediaModel(draft, model("minimax/h3/reference-to-video")).inputs,
  ).toEqual(draft.inputs);
});

test("shot results stay beside their shot even when the viewport is far away", () => {
  const p = fixture();
  p.production = { viewport: { x: 980, y: 660, scale: 0.18 } };
  const t = { ...task(), ownerId: "shot", inputs: [] };
  const position = resultPlacement(p, t);
  expect(position.x).toBe(527);
  expect(position.y).toBeLessThan(1000);
  const next = receiveProductionResult(p, asset("new-frame"), {
    ...p.nodes[1],
    canvasGeneration: { task: { ...t, position }, ...position },
  });
  const card = productionItems(next).find((n) => n.assetId === "new-frame")!;
  expect(card.x).toBe(position.x);
  expect(card.y).toBe(position.y);
});

test("overview generation stays near visible content instead of the remote viewport corner", () => {
  const p = fixture();
  p.production = { viewport: { x: 980, y: 660, scale: 0.18 } };
  const before = structuredClone(p);
  const pos = resultPlacement(
    p,
    { ...task(), inputs: [] },
    { width: 1650, height: 1100 },
  );
  expect(pos.x).toBeGreaterThan(0);
  expect(pos.y).toBeGreaterThanOrEqual(110);
  expect(pos.y).toBeLessThan(1000);
  expect(p).toEqual(before);
});

test("unattached results use the live view without changing the saved project viewport", () => {
  const p = fixture();
  p.nodes = [];
  const before = structuredClone(p);
  const view = { x: -1800, y: -500, scale: 0.5 };
  const position = resultPlacement(
    p,
    { ...task(), inputs: [] },
    undefined,
    view,
  );
  expect(position).toEqual({ x: 3664, y: 1160 });
  expect(p).toEqual(before);
});
