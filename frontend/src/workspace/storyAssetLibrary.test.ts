import { expect, test } from "bun:test";
import { asset, fixture } from "../production/fixtures.test-helper";
import { isLibraryAsset, setAssetUsage } from "./assetLibrary";
import {
  addStoryAsset,
  availableStoryMaterials,
  matchesAsset,
  storyAssets,
} from "./storyAssetLibrary";

test("asset workspace separates reviewed assets from generated candidates and excludes annotation copies", () => {
  const p = fixture();
  p.assets = [
    { ...asset("reviewed"), usage: "asset" },
    { ...asset("candidate"), usage: "asset", generated: true },
    { ...asset("removed"), usage: "asset", inLibrary: false },
    {
      ...asset("guide"),
      usage: "asset",
      annotationSourceId: "reviewed",
      inLibrary: true,
    },
    asset("material"),
    { ...asset("attachment"), attachmentOnly: true },
  ];
  expect(storyAssets(p).map((a) => a.id)).toEqual([
    "reviewed",
    "candidate",
    "removed",
  ]);
  expect(
    storyAssets(p)
      .filter(isLibraryAsset)
      .map((a) => a.id),
  ).toEqual(["reviewed"]);
  expect(availableStoryMaterials(p).map((a) => a.id)).toEqual(["material"]);
});

test("collecting a candidate preserves identity, selected image version, provenance and shot references", () => {
  const p = fixture();
  p.assets[0] = {
    ...p.assets[0],
    generated: true,
    usage: "asset",
    imageId: "image",
    imageVersionId: "chosen",
    inLibrary: false,
  };
  const next = addStoryAsset(p, "a");
  expect(next.assets[0]).toEqual({
    ...p.assets[0],
    inLibrary: true,
    inMaterials: false,
    assetStatus: "confirmed",
  });
  expect(next.nodes).toBe(p.nodes);
  expect(next.clips).toBe(p.clips);
  expect(storyAssets(next).filter(isLibraryAsset)).toHaveLength(1);
  expect(addStoryAsset(next, "a").assets).toHaveLength(p.assets.length);
  expect(storyAssets(setAssetUsage(next, "a", "material"))).toHaveLength(0);
});

test("adding an existing material collects it without a copy; missing files and annotations cannot be added", () => {
  const p = fixture();
  const added = addStoryAsset(p, "b");
  expect(added.assets.find((a) => a.id === "b")).toEqual({
    ...p.assets[1],
    usage: "asset",
    inLibrary: true,
    inMaterials: true,
    assetStatus: "confirmed",
  });
  p.assets[0].missing = true;
  p.assets[1].annotationSourceId = "a";
  for (const id of ["a", "b", "unknown"]) expect(addStoryAsset(p, id)).toBe(p);
});

test("asset search trims and ignores case, groups text with documents, and combines name and type", () => {
  const image = { ...asset("reference"), name: "Hero Portrait.PNG" };
  expect(matchesAsset(image, " PORTRAIT ", "image")).toBe(true);
  expect(matchesAsset(image, "portrait", "video")).toBe(false);
  expect(matchesAsset(image, "scene", "all")).toBe(false);
  expect(matchesAsset(asset("brief", "text"), "", "document")).toBe(true);
});
