import { expect, test } from "bun:test";
import { fixture, asset, model } from "./fixtures.test-helper";
import { createTask } from "./tasks";
import { addTaskReference, supportsReference } from "./referenceSelection";
import { saveTask } from "./document";
import { inputFor } from "./request";

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

test("missing, incompatible, short and excessive references are rejected", () => {
  const task = createTask(fixture(), [], "video");
  expect(
    supportsReference(asset("a"), model("minimax/h3/image-to-video")),
  ).toBe(false);
  expect(() =>
    addTaskReference(task, { ...asset("a"), missing: true }),
  ).toThrow();
  expect(() =>
    addTaskReference(task, { ...asset("v", "video"), duration: 1 }),
  ).toThrow();
  let draft = task;
  for (let i = 0; i < 12; i++)
    draft = { ...draft, ...addTaskReference(draft, asset(String(i))) };
  expect(() => addTaskReference(draft, asset("extra"))).toThrow("12");
  const video = addTaskReference(task, asset("v", "video")).inputs![0];
  expect([video.role, video.start, video.end]).toEqual([
    "video-reference",
    0,
    5,
  ]);
});
