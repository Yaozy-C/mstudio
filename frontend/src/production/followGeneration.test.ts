import { expect, test } from "bun:test";
import { followGeneration } from "./followGeneration";
import type { ProductionController } from "./useProduction";
import type { ProductionTask } from "./types";
test("image continuation resets an earlier video choice and uses the generated image", () => {
  const calls: unknown[] = [];
  const canvas = {
    items: [{ key: "result", assetId: "img", ownerId: "shot" }],
    act: (...args: unknown[]) => calls.push(args),
    focusShot: () => {},
  } as unknown as ProductionController;
  const task = {
    key: "run",
    ownerId: "shot",
    kind: "image",
    resultAssetId: "img",
    inputs: [],
  } as unknown as ProductionTask;
  followGeneration(canvas, task, "video");
  followGeneration(canvas, task);
  expect(calls).toEqual([
    ["video", ["result"]],
    ["image", ["result"]],
  ]);
});
