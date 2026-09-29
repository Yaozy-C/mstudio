import { expect, test } from "bun:test";
import { newProject } from "../model";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { writeScript } from "./scriptWrite";
import { splitParagraph, scriptChanged } from "./script";
import { restoreProject } from "../workspace/restoreProject";
const a = {
  id: "a",
  title: "Opening",
  action: "Open bag",
  dialogue: "Hello",
  sound: "Zip",
  duration: 4,
};
const b = { ...a, id: "b", title: "Pack", duration: 6 };
test("agent creates, edits, inserts, reorders, deletes and replaces structured script", () => {
  let p = applyOperations(newProject("Script"), 0, [
    {
      op: "add_node",
      id: "screenplay",
      kind: "screenplay",
      title: "Script",
      screenplay: { scriptMode: "replace", script: [a, b] },
    },
  ]);
  const edit = (screenplay: object) => {
    p = applyOperations(p, p.revision || 0, [
      { op: "update_node", id: "screenplay", screenplay },
    ]);
  };
  edit({
    script: [
      { id: "a", dialogue: "New line" },
      { ...a, id: "c" },
    ],
    paragraphOrder: ["c", "a", "b"],
  });
  expect(p.nodes[0].screenplay!.script!.map((s) => s.id)).toEqual([
    "c",
    "a",
    "b",
  ]);
  expect(p.nodes[0].screenplay!.script![1].duration).toBe(4);
  p = splitParagraph(p, "screenplay", "a", 1);
  p.nodes[1].resultAssetId = "video";
  p.nodes[1].shot!.frames = [{ assetId: "image", title: "Frame" }];
  const before = p;
  edit({ removeParagraphIds: ["a"] });
  expect(p.nodes[0].screenplay!.script!.map((s) => s.id)).toEqual(["c", "b"]);
  expect(p.nodes[1].resultAssetId).toBe("video");
  expect(p.nodes[1].shot!.frames![0].assetId).toBe("image");
  expect(scriptChanged(p, p.nodes[1])).toBe(true);
  expect(restoreProject(before, p).nodes[0].screenplay!.script).toEqual(
    before.nodes[0].screenplay!.script,
  );
  edit({ scriptMode: "replace", script: [{ ...b, action: "Rewritten" }] });
  expect(p.nodes[0].screenplay!.script).toEqual([
    { ...b, action: "Rewritten" },
  ]);
  expect(
    JSON.stringify(inspectProject(p, { nodeIds: ["screenplay"] })),
  ).toContain('"duration":6');
  edit({ scriptMode: "replace", script: [] });
  expect(p.nodes[0].screenplay!.script).toEqual([]);
  expect(p.nodes[1].resultAssetId).toBe("video");
});
test("invalid replacements and ambiguous deletions fail without partial mutation", () => {
  const old = [a, b];
  for (const patch of [
    { scriptMode: "replace", script: [{ id: "a", action: "partial" }] },
    { scriptMode: "replace" },
    { scriptMode: "typo" },
    { removeParagraphIds: ["unknown"] },
    { removeParagraphIds: ["a", "a"] },
    { script: [{ id: "a", action: "change" }], removeParagraphIds: ["a"] },
    { paragraphOrder: ["a"] },
    { paragraphOrder: ["a", "a"] },
    { scriptMode: "replace", script: [a], removeParagraphIds: ["b"] },
  ])
    expect(() => writeScript(old, patch)).toThrow();
  expect(old).toEqual([a, b]);
});

test("screen text roundtrips separately, invalidates linked shots and preserves legacy basis", () => {
  let p = applyOperations(newProject("AV script"), 0, [
    {
      op: "add_node",
      id: "screenplay",
      kind: "screenplay",
      title: "Script",
      screenplay: { script: [a] },
    },
  ]);
  p = splitParagraph(p, "screenplay", "a", 1);
  expect(scriptChanged(p, p.nodes[1])).toBe(false);
  const edit = (paragraph: object) => {
    p = applyOperations(p, p.revision || 0, [
      {
        op: "update_node",
        id: "screenplay",
        screenplay: { script: [paragraph] },
      },
    ]);
  };
  edit({ id: "a", onScreenText: "Everything in place" });
  expect(p.nodes[0].screenplay!.script![0].dialogue).toBe("Hello");
  expect(scriptChanged(p, p.nodes[1])).toBe(true);
  edit({ id: "a", sound: "Click" });
  const restored = JSON.parse(JSON.stringify(p));
  expect(restored.nodes[0].screenplay.script[0].onScreenText).toBe(
    "Everything in place",
  );
  expect(
    JSON.stringify(
      inspectProject(restored, {
        nodeIds: ["screenplay"],
        fields: ["screenplay"],
      }),
    ),
  ).toContain("Everything in place");
  expect(() =>
    writeScript([a], { script: [{ id: "a", onScreenText: 42 }] }),
  ).toThrow();
  expect(
    writeScript([a], { scriptMode: "replace", script: [a] })[0].onScreenText,
  ).toBeUndefined();
});
