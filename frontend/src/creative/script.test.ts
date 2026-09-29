import { expect, test } from "bun:test";
import { newProject } from "../model";
import {
  createScript,
  splitParagraph,
  updateParagraph,
  scriptChanged,
} from "./script";
import { restoreProject } from "../workspace/restoreProject";

test("independent script supports multiple linked shots and keeps media when changed", () => {
  let p = createScript(newProject("story"));
  const screenplay = p.nodes[0],
    id = screenplay.screenplay!.script![0].id;
  p = updateParagraph(p, screenplay.id, id, {
    action: "Open the bag",
    dialogue: "Hello",
  });
  p = splitParagraph(p, screenplay.id, id, 2);
  expect(p.nodes).toHaveLength(3);
  expect(p.nodes.slice(1).every((n) => n.shot?.scriptId === id)).toBe(true);
  expect(p.nodes[1].shot?.dialogue).toBe("");
  p.nodes[1].resultAssetId = "existing-video";
  p.nodes[1].shot!.frames = [{ assetId: "existing-frame", title: "Frame" }];
  const before = p;
  p = updateParagraph(p, screenplay.id, id, { action: "Close the bag" });
  expect(scriptChanged(p, p.nodes[1])).toBe(true);
  expect(scriptChanged(p, p.nodes[2])).toBe(true);
  expect(p.nodes[1].text).toBe("Open the bag");
  expect(p.nodes[1].resultAssetId).toBe("existing-video");
  expect(p.nodes[1].shot?.frames?.[0]?.assetId).toBe("existing-frame");
  expect(scriptChanged(restoreProject(before, p), before.nodes[1])).toBe(false);
  expect(splitParagraph(p, screenplay.id, id, 0)).toBe(p);
});
