import { expect, test } from "bun:test";
import { newProject, type Asset } from "../model";
import { restoreProject } from "./restoreProject";

test("restoring snapshots preserves concurrent assets and respects removed asset IDs", () => {
  const asset = (id: string): Asset => ({
    id,
    name: id,
    kind: "image",
    path: `/${id}.png`,
    preview: `/${id}.png`,
    duration: 0,
    width: 768,
    height: 1376,
    hasAudio: false,
  });
  const original = { ...newProject("test"), assets: [asset("a"), asset("b")] };
  const deleted = {
    ...original,
    assets: [asset("b")],
    removedAssetIds: ["a"],
    revision: 5,
  };
  const undone = restoreProject(original, deleted);
  expect(undone.assets).toEqual(original.assets);
  expect(undone.revision).toBe(6);
  const concurrent = { ...undone, assets: [...undone.assets, asset("new")] };
  const redone = restoreProject(deleted, concurrent);
  expect(redone.assets.map((a) => a.id)).toEqual(["b", "new"]);
  expect(redone.revision).toBe(7);
});
