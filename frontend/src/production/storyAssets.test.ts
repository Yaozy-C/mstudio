import { expect, test } from "bun:test";
import { asset, fixture } from "./fixtures.test-helper";
import { inspectProject } from "../assistant/inspectProject";
import { isLibraryAsset, setAssetUsage } from "../workspace/assetLibrary";
import { requestTask, runsOf } from "./requestTask";
import { createTask } from "./tasks";
import { receiveProductionResult } from "./document";

function setup() {
  const p = fixture();
  Object.assign(p.assets[0], {
    usage: "asset",
    imageId: "picture-a",
    imageVersionId: "selected",
    path: "/selected.png",
  });
  p.assets.push({ ...asset("unused"), usage: "asset" });
  const shot = p.nodes.find((n) => n.shot)!;
  shot.references = [
    { assetId: "a", purpose: "character identity" },
    { assetId: "b", purpose: "lighting mood" },
  ];
  const context = {
    callId: "call",
    turnId: "turn",
    turn: { projectId: p.id, models: {}, instruction: "生成分镜图" },
  };
  return {
    p,
    shot,
    context,
    op: { id: shot.id, mediaKind: "image", text: "Create shot" },
  };
}
const listed = (
  p: ReturnType<typeof fixture>,
  args: Record<string, unknown> = {},
) =>
  inspectProject(p, { section: "assets", ...args }).items as {
    id: string;
    usage: string;
  }[];

test("default discovery shows reviewed story assets; materials and drafts need explicit reads", () => {
  const { p } = setup();
  p.assets.push({ ...asset("draft"), usage: "asset", generated: true });
  expect(listed(p).map((a) => a.id)).toEqual(["a", "unused"]);
  expect(listed(p, { ids: [] }).map((a) => a.id)).toEqual(["a", "unused"]);
  expect(listed(p, { usage: "material" }).map((a) => a.id)).toEqual([
    "a",
    "reference",
    "unused",
  ]);
  expect(listed(p, { ids: ["b", "draft"] }).map((a) => a.id)).toEqual([
    "b",
    "draft",
  ]);
  expect(listed(p, { usage: "all" })).toHaveLength(3);
  expect(inspectProject(p, {}).assetCount).toBe(2);
});

test("a new storyboard defaults to this shot's assets, without injecting library assets or ordinary materials", () => {
  const { p, op, context } = setup();
  const task = runsOf(requestTask(p, op, context))[0];
  expect(task.inputs.map((i) => i.assetId)).toEqual(["a"]);
  expect(task.inputs[0].purpose).toBe("character identity");
  expect(p.assets[0].imageVersionId).toBe("selected");
  expect(p.assets[0].path).toBe("/selected.png");
  expect(
    runsOf(requestTask(p, { ...op, references: [] }, context))[0].inputs,
  ).toEqual([]);
  const explicit = runsOf(
    requestTask(
      p,
      { ...op, references: [{ assetId: "b", purpose: "lighting only" }] },
      context,
    ),
  )[0];
  expect(explicit.inputs.map((i) => i.assetId)).toEqual(["b"]);
  expect(() => requestTask(p, { ...op, references: null }, context)).toThrow();
  p.assets[0].missing = true;
  expect(() => requestTask(p, op, context)).toThrow("不可用");
});

test("explicit video frames and ordinary task inputs survive; changing usage preserves IDs, versions and shot/timeline references", () => {
  const { p, shot, context } = setup();
  const source = {
    ...createTask(p, [], "video"),
    ownerId: shot.id,
    mode: "single" as const,
    inputs: [
      {
        key: "frame",
        assetId: "b",
        role: "first-frame" as const,
        purpose: "selected frame",
      },
    ],
  };
  const task = runsOf(
    requestTask(
      p,
      { id: shot.id, mediaKind: "video", text: "Animate" },
      { ...context, turn: { ...context.turn, task: source } },
    ),
  )[0];
  expect(task.inputs).toEqual(source.inputs);
  expect(task.mode).toBe("single");
  const changed = setAssetUsage(p, "a", "material");
  expect(changed.assets[0]).toEqual({
    ...p.assets[0],
    usage: "material",
    inMaterials: true,
    assetStatus: undefined,
    inLibrary: undefined,
  });
  expect(changed.nodes).toBe(p.nodes);
  expect(changed.clips).toBe(p.clips);
  expect(isLibraryAsset(changed.assets[0])).toBe(true);
  expect(listed(changed).map((a) => a.id)).toEqual(["unused"]);
});

test("asset-generation candidates keep their purpose without automatic review; shot output remains material", () => {
  const { p, context } = setup();
  for (const purpose of ["asset", undefined] as const) {
    const run = runsOf(
      requestTask(
        p,
        {
          mediaKind: "image",
          text: "Generate",
          generationPurpose: purpose,
          references: [],
        },
        context,
      ),
    )[0];
    const result = asset("result");
    const next = receiveProductionResult(p, result, {
      id: "source",
      kind: "asset",
      title: "source",
      text: "",
      x: 0,
      y: 0,
      canvasGeneration: { task: run, x: 0, y: 0 },
    });
    const saved = next.assets.find((a) => a.id === "result")!;
    expect(saved.usage).toBe(purpose ?? "material");
    expect(isLibraryAsset(saved)).toBe(false);
  }
});
