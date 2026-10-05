import { expect, test } from "bun:test";
import { fixture } from "../production/fixtures.test-helper";
import { collectAsset, uncollectAsset } from "../workspace/assetLibrary";
import { applyOperations, inspectProject } from "./projectCommands";
import { requestTask } from "../production/requestTask";
import { createTask } from "../production/tasks";

test("removed media is absent from agent discovery, including explicit ID lookup", () => {
  const p = uncollectAsset(fixture(), "a");
  for (const args of [
    { section: "assets" },
    { section: "assets", ids: ["a"] },
  ]) {
    expect(JSON.stringify(inspectProject(p, args))).not.toContain('"id":"a"');
  }
  expect(
    JSON.stringify(
      inspectProject(collectAsset(p, p.assets[0]), { section: "assets" }),
    ),
  ).toContain('"id":"a"');
});

test("stale IDs cannot add references or timeline uses, but existing references survive edits", () => {
  const p = uncollectAsset(fixture(), "a");
  for (const op of [
    {
      op: "set_references",
      id: "shot",
      references: [{ assetId: "a", purpose: "old" }],
    },
    {
      op: "add_node",
      id: "new",
      kind: "asset",
      title: "Old asset",
      assetId: "a",
    },
    { op: "append_clip", assetId: "a" },
  ])
    expect(() => applyOperations(p, [op])).toThrow("移除");
  const oldReference = uncollectAsset(fixture(), "reference");
  const edited = applyOperations(oldReference, [
    {
      op: "set_references",
      id: "shot",
      references: [
        { assetId: "reference", purpose: "Updated existing purpose" },
      ],
    },
  ]);
  expect(
    edited.nodes.find((n) => n.id === "shot")?.references?.[0].assetId,
  ).toBe("reference");
  expect(edited.assets).toBe(oldReference.assets);
});

test("agent generation rejects both explicit and inherited removed references", () => {
  const p = uncollectAsset(fixture(), "reference");
  const context = {
    callId: "removed",
    turnId: "turn",
    turn: {
      projectId: p.id,
      models: { image: "image" },
      instruction: "Generate",
    },
  };
  const op = { mediaKind: "image", id: "shot", text: "Generate a frame" };
  expect(() => requestTask(p, op, context)).toThrow("移除");
  expect(() =>
    requestTask(
      p,
      { ...op, references: [{ assetId: "reference", purpose: "old" }] },
      context,
    ),
  ).toThrow("移除");
  const task = createTask(p, [], "image");
  task.inputs = [
    { key: "old", assetId: "reference", role: "reference", purpose: "old" },
  ];
  task.ownerId = "shot";
  expect(() =>
    requestTask(
      p,
      { ...op, canvasTaskKey: task.key },
      {
        ...context,
        turn: { ...context.turn, task },
      },
    ),
  ).toThrow("移除");
});
