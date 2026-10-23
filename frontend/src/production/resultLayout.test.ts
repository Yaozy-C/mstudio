import { expect, test } from "bun:test";
import { newProject } from "../model";
import { asset, fixture } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { receiveProductionResult } from "./document";
import { productionItems } from "./items";
import { resultPlacement } from "./resultPlacement";
import { overlapsResult, resultSize } from "./resultLayout";
import type { ProductionTask } from "./types";
import { SHOT_CARD_WIDTH } from "./shotLayout";

test("Agent batches reserve the sent canvas coordinates and retain order when results arrive backwards", () => {
  let p = newProject("placement");
  const context = {
    callId: "batch",
    turnId: "turn",
    turn: {
      projectId: p.id,
      models: { image: "image", execution: "automatic" as const },
      instruction: "three views",
      canvas: {
        view: { x: -1800, y: -500, scale: 0.5 },
        size: { width: 1600, height: 900 },
      },
    },
  };
  for (let i = 0; i < 3; i++)
    p = requestTask(
      p,
      { mediaKind: "image", title: `View ${i}`, text: `view ${i}` },
      context,
      i,
    );
  const runs = runsOf(p);
  expect(runs[0].position).toEqual({ x: 3664, y: 1160 });
  expect(new Set(runs.map((run) => JSON.stringify(run.position))).size).toBe(3);
  // The saved project view is stale, and can change again while jobs run.
  p.viewport = { x: 8000, y: 2000, scale: 0.18 };
  for (const run of [...runs].reverse()) {
    p = receiveProductionResult(p, asset(run.key), {
      id: "source",
      kind: "asset",
      title: "",
      text: "",
      x: 0,
      y: 0,
      canvasGeneration: { task: run, ...run.position! },
    });
  }
  for (const run of runs) {
    const card = productionItems(p).find((item) => item.assetId === run.key)!;
    expect({ x: card.x, y: card.y }).toEqual(run.position!);
  }
});

test("targeted Agent tasks anchor to the shot without assigning result ownership", () => {
  const p = fixture();
  const task: ProductionTask = {
    key: "new",
    kind: "image",
    mode: "multi",
    inputs: [],
    prompt: "new",
    modelId: "image",
    targetNodeId: "shot",
  };
  const position = resultPlacement(p, task, undefined, {
    x: -9000,
    y: -9000,
    scale: 1,
  });
  expect(position.x).toBe(35 + SHOT_CARD_WIDTH + 32);
  expect(position.y).toBeLessThan(1000);
  expect(task.ownerId).toBeUndefined();
});

test("video placement avoids overlap at the extra ten pixels of card width", () => {
  let p = newProject("video");
  p.assets = [asset("obstacle")];
  p.nodes = [
    {
      id: "obstacle",
      kind: "asset",
      assetId: "obstacle",
      title: "",
      text: "",
      x: 255,
      y: 0,
    },
  ];
  const task: ProductionTask = {
    key: "video",
    title: "Video result",
    kind: "video",
    mode: "multi",
    inputs: [],
    prompt: "new",
    modelId: "video",
    position: { x: 0, y: 0 },
  };
  p = receiveProductionResult(p, asset("video", "video"), {
    id: "source",
    kind: "asset",
    title: "",
    text: "",
    x: 0,
    y: 0,
    canvasGeneration: { task, x: 0, y: 0 },
  });
  const cards = productionItems(p);
  expect(
    overlapsResult(
      cards.find((n) => n.assetId === "video")!,
      resultSize("video"),
      cards.find((n) => n.assetId === "obstacle")!,
    ),
  ).toBe(false);
});

test("headless generation reserves a default position when no viewport was saved", () => {
  const p = newProject("headless");
  // Native fixtures and older documents may omit UI state entirely.
  Reflect.deleteProperty(p, "viewport");
  const next = requestTask(
    p,
    { mediaKind: "image", title: "New image", text: "new image" },
    {
      callId: "headless",
      turnId: "turn",
      turn: { projectId: p.id, models: {}, instruction: "new image" },
    },
  );
  expect(runsOf(next)[0].position).toEqual({ x: 32, y: 80 });
});

test("shot images and videos reserve their result areas without a viewport", () => {
  const p = fixture();
  Reflect.deleteProperty(p, "viewport");
  const task: ProductionTask = {
    key: "shot-result",
    kind: "image",
    mode: "multi",
    inputs: [],
    targetNodeId: "shot",
    prompt: "new",
    modelId: "image",
  };
  const image = resultPlacement(p, task);
  const video = resultPlacement(p, { ...task, kind: "video" });
  expect(image.x).toBe(35 + SHOT_CARD_WIDTH + 32);
  expect(video.x).toBe(35 + SHOT_CARD_WIDTH + 640);
  expect(Number.isFinite(image.y) && Number.isFinite(video.y)).toBe(true);
});

test("a user card occupying a reserved slot moves only the incoming result", () => {
  let p = newProject("occupied");
  const next = requestTask(
    p,
    { mediaKind: "image", title: "New image", text: "new image" },
    {
      callId: "reserved",
      turnId: "turn",
      turn: { projectId: p.id, models: {}, instruction: "new image" },
    },
  );
  const task = runsOf(next)[0];
  const reserved = task.position!;
  p = {
    ...next,
    assets: [asset("user")],
    nodes: [
      {
        id: "user",
        kind: "asset",
        assetId: "user",
        title: "user",
        text: "",
        ...reserved,
      },
    ],
  };
  const source = {
    id: "source",
    kind: "asset" as const,
    title: "",
    text: "",
    x: 0,
    y: 0,
    canvasGeneration: { task, ...reserved },
  };
  p = receiveProductionResult(p, asset("result"), source);
  const items = productionItems(p);
  const user = items.find((n) => n.assetId === "user")!;
  const result = items.find((n) => n.assetId === "result")!;
  expect({ x: user.x, y: user.y }).toEqual(reserved);
  expect(result.x).toBe(reserved.x);
  expect(result.y).toBeGreaterThan(reserved.y);
  // Later imports must preserve the user's new position for the result too.
  p.production!.positions![result.key] = { x: 6000, y: 3000 };
  const repeated = receiveProductionResult(p, asset("result"), source);
  expect(
    productionItems(repeated).find((n) => n.key === result.key),
  ).toMatchObject({ x: 6000, y: 3000 });
});
