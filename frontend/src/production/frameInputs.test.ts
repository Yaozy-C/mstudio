import { expect, test } from "bun:test";
import { fixture, model } from "./fixtures.test-helper";
import { frameRoles, selectMediaModel, setFrameInput } from "./frameInputs";
import { inputFor } from "./request";
import { withMode } from "./tasks";
import type { ProductionTask } from "./types";
const base = (): ProductionTask => ({
  key: "t",
  kind: "video",
  mode: "ends",
  modelId: "model",
  prompt: "推进",
  inputs: [
    { key: "a", assetId: "a", role: "first-frame", purpose: "" },
    { key: "b", assetId: "b", role: "last-frame", purpose: "" },
  ],
});
test("replacing first frame preserves last frame and submission roles regardless of order", () => {
  const p = fixture();
  const task = {
    ...base(),
    ...setFrameInput(base(), p, "first-frame", {
      key: "reference",
      assetId: "reference",
      role: "reference",
      purpose: "",
    }),
  };
  const submitted = withMode(task, "ends");
  const result = inputFor(p, submitted, model("minimax/h3/image-to-video"), [
    {
      assetId: "reference",
      kind: "image",
      url: "https://example.com/first.png",
    },
    { assetId: "b", kind: "image", url: "https://example.com/last.png" },
  ]);
  expect(result.image_url).toBe("https://example.com/first.png");
  expect(result.end_image_url).toBe("https://example.com/last.png");
});
test("frame slots reject videos and duplicates without changing either slot", () => {
  const task = base();
  for (const id of ["v", "b"])
    expect(() =>
      setFrameInput(task, fixture(), "first-frame", {
        key: id,
        assetId: id,
        role: "reference",
        purpose: "",
      }),
    ).toThrow();
  expect(task).toEqual(base());
  expect(setFrameInput(task, fixture(), "last-frame").mode).toBe("single");
});
test("switching to frames never promotes video references or infers frames from order", () => {
  const task = base();
  task.inputs = [
    {
      key: "v",
      assetId: "v",
      role: "video-reference",
      purpose: "",
      start: 0,
      end: 5,
    },
    { key: "a", assetId: "a", role: "reference", purpose: "" },
  ];
  const next = selectMediaModel(task, model("minimax/h3/image-to-video"));
  expect(next.inputs).toEqual(task.inputs);
  expect(next.mode).toBe("single");
  expect(frameRoles(model("minimax/h3-max/image-to-video"))).toEqual([
    "first-frame",
  ]);
});
