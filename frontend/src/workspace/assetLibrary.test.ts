import { expect, test } from "bun:test";
import { newProject, type Asset } from "../model";
import { collectAsset, isLibraryAsset, uncollectAsset } from "./assetLibrary";

const upload: Asset = {
  id: "upload",
  name: "生成结果-用户文件.png",
  kind: "image",
  path: "/upload.png",
  preview: "",
  duration: 1,
  width: 100,
  height: 100,
  hasAudio: false,
};
const result: Asset = { ...upload, id: "result", generated: true };

test("library distinguishes provenance, not filenames, and generated results need explicit collection", () => {
  let p = { ...newProject("test"), assets: [upload, result] };
  expect(p.assets.filter(isLibraryAsset).map((a) => a.id)).toEqual(["upload"]);
  p = collectAsset(p, result);
  expect(p.assets.filter(isLibraryAsset).map((a) => a.id)).toEqual([
    "upload",
    "result",
  ]);
  expect(p.assets[1].generated).toBe(true);
  expect(collectAsset(p, result).assets).toHaveLength(2);
});

test("removing library membership preserves existing usages and can be collected again", () => {
  const p = {
    ...newProject("test"),
    assets: [upload],
    nodes: [
      {
        id: "node",
        kind: "asset" as const,
        assetId: upload.id,
        title: "ref",
        text: "",
        x: 0,
        y: 0,
        width: 200,
        height: 100,
      },
    ],
  };
  const removed = uncollectAsset(p, upload.id);
  expect(removed.assets.filter(isLibraryAsset)).toHaveLength(0);
  expect(removed.nodes).toBe(p.nodes);
  expect(removed.clips).toBe(p.clips);
  expect(removed.assets[0].path).toBe(upload.path);
  expect(
    collectAsset(removed, upload).assets.filter(isLibraryAsset),
  ).toHaveLength(1);
});
