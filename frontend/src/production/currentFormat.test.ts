import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { framesOf } from "./frames";
import { takesOf } from "../creative/document";
import { applyOperations } from "../assistant/projectCommands";
import { receiveGeneratedResult } from "../creative/receiveGeneratedResult";
import type { ProductionSource } from "./types";

test("retired generation commands and payloads are rejected without changing the document", () => {
  const p = fixture();
  const original = JSON.stringify(p);
  expect(() =>
    applyOperations(p, 0, [{ op: "prepare_generation", id: "shot" }]),
  ).toThrow("不支持");
  expect(() =>
    receiveGeneratedResult(p, asset("new"), p.nodes[1] as ProductionSource),
  ).toThrow("上下文");
  expect(() =>
    applyOperations(p, 0, [
      { op: "update_node", id: "shot", shot: { frameAssetId: "a" } },
    ]),
  ).toThrow("不支持");
  expect(JSON.stringify(p)).toBe(original);
});

test("frames and takes are read only from their canonical lists", () => {
  const p = fixture();
  const node = {
    ...p.nodes[1],
    resultAssetId: "v",
    shot: { ...p.nodes[1].shot!, frames: [], takes: [], frameAssetId: "a" },
  };
  expect(framesOf(node)).toEqual([]);
  expect(takesOf(node)).toEqual([]);
});

test("Agent edits frame descriptions while rejecting unsupported fields, wrong media and duplicate inputs", () => {
  let p = fixture();
  const original = JSON.stringify(p);
  for (const frames of [
    [
      {
        assetId: "a",
        title: "A",
        unsupported: true,
      },
    ],
    [{ assetId: "v", title: "Video is not a frame" }],
    [
      { assetId: "a", title: "A" },
      { assetId: "a", title: "Duplicate" },
    ],
  ])
    expect(() =>
      applyOperations(p, 0, [
        { op: "update_node", id: "shot", shot: { frames } },
      ]),
    ).toThrow();
  expect(JSON.stringify(p)).toBe(original);
  p = applyOperations(p, 0, [
    {
      op: "update_node",
      id: "shot",
      shot: { frames: [{ assetId: "a", title: "A", prompt: "Changed" }] },
    },
  ]);
  expect(p.nodes[1].shot!.frames).toEqual([
    { assetId: "a", title: "A", prompt: "Changed" },
  ]);
  expect(p.assets).toHaveLength(4);
});
