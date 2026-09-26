import { test, expect } from "bun:test";
import { newProject, type Asset } from "../model";
import { applyFileStatus } from "./useMissingAssets";

test("missing files retain project content and restored files clear the placeholder", () => {
  const asset: Asset = {
    id: "a",
    name: "test.png",
    path: "/files/test.png",
    preview: "/files/preview.jpg",
    kind: "image",
    width: 100,
    height: 100,
    duration: 1,
    hasAudio: false,
  };
  const p = {
    ...newProject("test"),
    assets: [asset],
    clips: [
      {
        id: "c",
        assetId: "a",
        trimIn: 0,
        trimOut: 1,
        speed: 1,
        volume: 1,
        start: 0,
        trackId: "v1",
      },
    ],
  };
  const missing = applyFileStatus(p, { a: true });
  expect(missing.assets[0]).toEqual({ ...asset, missing: true });
  expect(missing.clips).toBe(p.clips);
  expect(missing.nodes).toBe(p.nodes);
  expect(applyFileStatus(missing, { a: true })).toBe(missing);
  expect(applyFileStatus(missing, { a: false }).assets[0].missing).toBe(false);
});
