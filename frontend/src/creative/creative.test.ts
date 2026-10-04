import { expect, test } from "bun:test";
import { newProject, duration, type Asset } from "../model";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { addShotResult, shotBasis } from "./document";
import { assembleScreenplay, chooseTake } from "./timeline";
import { removeNodes } from "../canvas/removeNodes";
import { describeAttachment } from "../assistant/attachments";
const asset = (id: string, seconds = 6): Asset => ({
  id,
  name: id,
  kind: "video",
  path: id,
  preview: id,
  duration: seconds,
  width: 100,
  height: 100,
  hasAudio: true,
});
function fixture() {
  return applyOperations(
    {
      ...newProject("film"),
      assets: [asset("a"), asset("b"), asset("c", 0.1)],
    },
    [
      {
        op: "add_node",
        id: "screenplay",
        kind: "screenplay",
        title: "Film",
        text: "",
        screenplay: { script: [] },
      },
      {
        op: "add_node",
        id: "s1",
        kind: "shot",
        title: "Open",
        text: "Open the bag",
        shot: {
          screenplayId: "screenplay",
          order: 1,
          duration: 6,
          dialogue: "Let's pack",
        },
        resultAssetId: "a",
      },
      {
        op: "add_node",
        id: "s2",
        kind: "shot",
        title: "Close",
        text: "Close the bag",
        shot: {
          screenplayId: "screenplay",
          order: 2,
          duration: 3,
          dialogue: "Ready",
        },
        resultAssetId: "b",
      },
    ],
  );
}
test("one screenplay owns paired shots; partial edits keep dialogue, identity and mark existing visuals stale", () => {
  const p = fixture();
  const q = applyOperations(p, [
    { op: "update_node", id: "s1", text: "Open faster" },
  ]);
  expect(q.nodes[1].shot?.dialogue).toBe("Let's pack");
  expect(q.nodes[1].shot?.visualChanged).toBe(true);
  expect(q.nodes[2]).toEqual(p.nodes[2]);
  const info = inspectProject(q, {
    nodeIds: ["s1"],
    fields: ["shot", "dialogue"],
  });
  expect(JSON.stringify(info)).toContain("Let's pack");
  expect(removeNodes(q, ["screenplay"]).nodes).toHaveLength(0);
});
test("reject invalid associations and duplicate shot order atomically", () => {
  const p = fixture(),
    original = JSON.stringify(p);
  for (const shot of [
    { screenplayId: "missing" },
    { order: 2 },
    { duration: -1 },
    { frames: [{ assetId: "a", title: "frame" }] },
  ]) {
    expect(() =>
      applyOperations(p, [
        { op: "update_node", id: "s1", text: "changed", shot },
      ]),
    ).toThrow();
    expect(JSON.stringify(p)).toBe(original);
  }
  expect(
    applyOperations(p, [{ op: "remove_node", id: "s1" }]).nodes.some(
      (n) => n.id === "s1",
    ),
  ).toBe(false);
});
test("assembly matches planned durations and does not duplicate already edited shots", () => {
  const p = assembleScreenplay(fixture(), "screenplay");
  expect(p.clips.map((c) => duration(c))).toEqual([6, 3]);
  expect(p.clips.map((c) => c.shotId)).toEqual(["s1", "s2"]);
  expect(p.clips[1].start).toBe(6);
  expect(assembleScreenplay(p, "screenplay")).toBe(p);
});
test("generation results stay under their shot and never silently replace chosen media", () => {
  const p = fixture(),
    node = p.nodes[1];
  const q = addShotResult(p, asset("new"), node.id, node);
  expect(q.nodes).toHaveLength(p.nodes.length);
  expect(q.nodes[1].resultAssetId).toBe("a");
  expect(q.nodes[1].shot?.takes?.at(-1)?.basis).toBe(shotBasis(node));
  const revised = applyOperations(q, [
    { op: "update_node", id: "s1", text: "faster" },
  ]);
  expect(chooseTake(revised, "s1", "new").nodes[1].shot?.visualChanged).toBe(
    true,
  );
});
test("replace a split shot without changing timings, captions, other shots or original audio", () => {
  let p = assembleScreenplay(fixture(), "screenplay");
  const c = p.clips[0],
    untouched = p.clips[1];
  p = {
    ...p,
    captions: [{ id: "cap", start: 1, end: 2, text: "keep" }],
    clips: [
      { ...c, trimOut: 2 },
      { ...c, id: "split", trimIn: 2, start: 2 },
      untouched,
    ],
  };
  p = addShotResult(p, asset("new", 9), "s1", p.nodes[1]);
  const q = chooseTake(p, "s1", "new");
  expect(
    q.clips.slice(0, 2).map((c) => [c.start, duration(c), c.volume]),
  ).toEqual([
    [0, 2, 0],
    [2, 4, 0],
  ]);
  expect(q.clips[2]).toEqual(untouched);
  expect(q.captions).toEqual(p.captions);
  expect(
    q.clips
      .slice(3)
      .map((c) => [c.assetId, c.trimIn, c.trimOut, c.start, c.volume]),
  ).toEqual([
    ["a", 0, 2, 0, 1],
    ["a", 2, 6, 2, 1],
  ]);
  expect(chooseTake(q, "s1", "new").clips).toHaveLength(5);
  expect(describeAttachment(q, { kind: "clip", id: "split" })?.title).toContain(
    "2–6s",
  );
  const bad = addShotResult(q, asset("too-short", 0.1), "s1");
  expect(() => chooseTake(bad, "s1", "too-short")).toThrow();
});

test("choosing a new video does not pretend an old storyboard image has updated", () => {
  let p = fixture();
  p = { ...p, assets: [...p.assets, { ...asset("frame"), kind: "image" }] };
  p = applyOperations(p, [
    {
      op: "update_node",
      id: "s1",
      shot: { frames: [{ assetId: "frame", title: "frame" }] },
    },
  ]);
  const originalFrames = p.nodes[1].shot!.frames;
  p = applyOperations(p, [
    { op: "update_node", id: "s1", text: "different action" },
  ]);
  p = addShotResult(p, asset("fresh"), "s1", p.nodes[1]);
  const q = chooseTake(p, "s1", "fresh");
  expect(q.nodes[1].shot?.visualChanged).toBe(false);
  expect(q.nodes[1].shot!.frames).toEqual(originalFrames);
});
