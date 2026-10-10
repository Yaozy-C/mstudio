import { expect, test } from "bun:test";
import { fixture, asset, model } from "./fixtures.test-helper";
import { createTask } from "./tasks";
import { addTaskReference, supportsReference } from "./referenceSelection";
import { saveTask } from "./document";
import { inputFor } from "./request";
import { collectAsset, uncollectAsset } from "../workspace/assetLibrary";

test("removed assets cannot be picked again until restored, while generated images remain selectable", () => {
  const p = uncollectAsset(fixture(), "a");
  const removed = p.assets.find((a) => a.id === "a")!;
  expect(supportsReference(removed)).toBe(false);
  expect(() => addTaskReference(createTask(p, [], "image"), removed)).toThrow();
  expect(p.assets.find((a) => a.id === "a")?.path).toBe("/a");
  expect(supportsReference({ ...asset("generated"), generated: true })).toBe(
    true,
  );
  expect(supportsReference(collectAsset(p, removed).assets[0])).toBe(true);
});

test("task reference edits stay local until saved and reach model input", () => {
  const p = fixture(),
    m = model("minimax/h3/reference-to-video");
  const original = {
    ...createTask(p, [], "video"),
    key: "run:test",
    prompt: "Open the bag",
  };
  let draft = { ...original, ...addTaskReference(original, p.assets[0], m) };
  draft = { ...draft, ...addTaskReference(draft, p.assets[1], m) };
  expect(original.inputs).toHaveLength(0);
  expect(addTaskReference(draft, p.assets[0], m)).toEqual({});
  draft = { ...draft, inputs: draft.inputs.filter((r) => r.assetId !== "a") };
  const saved = saveTask(p, draft).production!.drafts![draft.key];
  expect(saved.inputs.map((r) => r.assetId)).toEqual(["b"]);
  const encoded = inputFor(p, saved, m, [
    { assetId: "b", kind: "image", url: "https://example.com/b.png" },
  ]);
  expect(JSON.stringify(encoded)).toContain("https://example.com/b.png");
  expect(p.production?.drafts?.[draft.key]).toBeUndefined();
});

test("references reject missing assets and use configured count limits", () => {
  const task = createTask(fixture(), [], "video");
  expect(
    supportsReference(asset("a"), model("minimax/h3/image-to-video")),
  ).toBe(false);
  expect(() =>
    addTaskReference(task, { ...asset("a"), missing: true }),
  ).toThrow();
  expect(() =>
    addTaskReference(task, { ...asset("v", "video"), duration: 1 }),
  ).not.toThrow();
  let draft = task;
  for (let i = 0; i < 12; i++)
    draft = { ...draft, ...addTaskReference(draft, asset(String(i))) };
  expect(addTaskReference(draft, asset("extra")).inputs).toHaveLength(13);
  expect(() =>
    addTaskReference(
      draft,
      asset("extra"),
      model("custom", "video", {
        references: [
          { key: "/images", kind: "image", role: "reference", multiple: true },
        ],
        referenceLimit: 12,
      }),
    ),
  ).toThrow("12");
  const video = addTaskReference(task, asset("v", "video")).inputs![0];
  expect([video.role, video.start, video.end]).toEqual([
    "video-reference",
    0,
    20,
  ]);
});

test("video references retain full duration across pickers and selected canvas items", async () => {
  const { attachmentInput } = await import("./attachmentInput");
  const { productionItems } = await import("./items");
  const p = fixture();
  p.assets.find((a) => a.id === "v")!.duration = 17.033333;
  p.nodes.push({
    id: "video-card",
    kind: "asset",
    assetId: "v",
    title: "Video",
    text: "",
    x: 0,
    y: 0,
  });
  const task = createTask(p, [], "video");
  const attached = attachmentInput(p, { kind: "asset", id: "v" })!;
  const picked = addTaskReference(
    task,
    p.assets.find((a) => a.id === "v")!,
  ).inputs![0];
  const selected = createTask(
    p,
    productionItems(p).filter((i) => i.nodeId === "video-card"),
    "video",
  ).inputs[0];
  for (const input of [attached, picked, selected]) {
    expect(input.start).toBe(0);
    expect(input.end).toBe(17.033333);
  }
  const m = model("minimax/h3/reference-to-video");
  expect(() =>
    inputFor(p, { ...task, prompt: "Follow reference", inputs: [attached] }, m),
  ).toThrow("15 秒");
  expect(() =>
    inputFor(
      p,
      {
        ...task,
        prompt: "Follow reference",
        inputs: [{ ...attached, start: 2.1, end: 17 }],
      },
      m,
    ),
  ).not.toThrow();
});
