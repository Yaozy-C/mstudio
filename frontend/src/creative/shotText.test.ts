import { expect, test } from "bun:test";
import { newProject, type BoardNode } from "../model";
import { restoreProject } from "../workspace/restoreProject";
import { actionText, editShotText } from "./shotText";

function fixture(text = "打开餐盒") {
  const node: BoardNode = {
    id: "shot",
    kind: "shot",
    title: "装入午餐",
    x: 0,
    y: 0,
    text,
    resultAssetId: "video",
    references: [{ assetId: "ref", purpose: "餐盒外观" }],
    shot: {
      screenplayId: "screenplay",
      order: 1,
      duration: 4,
      dialogue: "Let's pack.",
      frames: [{ assetId: "frame", title: "Frame" }],
      takes: [{ assetId: "video", trimIn: 1, trimOut: 5 }],
    },
  };
  return { ...newProject("copy editing"), nodes: [node] };
}

test("manual action edits preserve dialogue, chosen media, other nodes and the cut; undo restores stale flags", () => {
  const p = fixture();
  const q = editShotText(
    p,
    "shot",
    "action",
    "打开餐盒\n镜头推近。\n放入午餐。",
  );
  expect(actionText(q.nodes[0])).toBe("打开餐盒\n镜头推近。\n放入午餐。");
  expect(q.nodes[0].shot).toEqual({
    ...p.nodes[0].shot!,
    visualChanged: true,
  });
  expect(q.nodes[0].resultAssetId).toBe("video");
  expect(q.nodes[0].references).toBe(p.nodes[0].references);
  expect(q.clips).toBe(p.clips);
  expect(q.assets).toBe(p.assets);
  expect(restoreProject(p, q).nodes).toEqual(p.nodes);
});

test("clearing either field never replaces it with a placeholder or destroys adjoining notes", () => {
  let p = fixture();
  p = editShotText(p, "shot", "action", "");
  p = editShotText(p, "shot", "dialogue", "");
  expect(p.nodes[0].text).toBe("");
  expect(actionText(p.nodes[0])).toBe("");
  expect(p.nodes[0].shot?.dialogue).toBe("");
  const q = editShotText(p, "shot", "action", "重新打开餐盒");
  expect(actionText(q.nodes[0])).toBe("重新打开餐盒");
  expect(q.nodes[0].shot?.dialogue).toBe("");
});

test("unchanged edits and a pending edit for a deleted shot do not create save or undo work", () => {
  const p = fixture();
  expect(editShotText(p, "shot", "action", "打开餐盒")).toBe(p);
  expect(editShotText(p, "shot", "dialogue", "Let's pack.")).toBe(p);
  expect(editShotText(p, "deleted", "action", "edit")).toBe(p);
  const q = editShotText(p, "shot", "dialogue", "Ready.");
  expect(q.nodes[0].text).toBe(p.nodes[0].text);
  expect(q.nodes[0].shot?.dialogue).toBe("Ready.");
});
