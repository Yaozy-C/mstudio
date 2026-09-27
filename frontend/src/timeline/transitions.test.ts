import { expect, test } from "bun:test";
import { defaultDesign } from "./transitionDesign";
import { newProject, makeClip, type Asset } from "../model";
import { attachmentInput, sameInput } from "../production/attachmentInput";
import { taskAttachments } from "../production/chat";
import { applyOperations } from "../assistant/projectCommands";
import { setTransition, seams, transitionKinds } from "./transitions";
const asset: Asset = {
  id: "v",
  name: "v",
  kind: "video",
  path: "",
  preview: "",
  duration: 6,
  width: 320,
  height: 240,
  hasAudio: true,
};
function fixture() {
  const p = newProject("transitions");
  return {
    ...p,
    assets: [asset],
    clips: [
      { ...makeClip(asset), id: "left", start: 0, trimIn: 0, trimOut: 3 },
      { ...makeClip(asset), id: "right", start: 3, trimIn: 3, trimOut: 6 },
      { ...makeClip(asset), id: "audio", trackId: "a1", start: 0 },
    ],
  };
}
test("Agent adds/removes all transition types without moving edits or sound", () => {
  const p = fixture();
  expect(seams(p, "v1")).toHaveLength(1);
  for (const kind of Object.keys(
    transitionKinds,
  ) as (keyof typeof transitionKinds)[]) {
    const next = applyOperations(p, 0, [
      {
        op: "set_transition",
        fromClipId: "left",
        id: "right",
        kind,
        duration: 0.5,
        ...(kind === "custom" ? { design: defaultDesign() } : {}),
      },
    ]);
    expect(next.clips[1].transition).toEqual({
      fromClipId: "left",
      kind,
      duration: 0.5,
      ...(kind === "custom" ? { design: defaultDesign() } : {}),
    });
    expect(next.clips.map(({ transition: _, ...clip }) => clip)).toEqual(
      p.clips,
    );
    expect(next.captions).toEqual(p.captions);
    expect(
      setTransition(next, "left", "right", null, 0).clips[1].transition,
    ).toBeUndefined();
  }
  expect(p.clips[1].transition).toBeUndefined();
});
test("reject stale edits, gaps, overlays, unsupported types and invalid durations", () => {
  const p = fixture();
  for (const seconds of [NaN, Infinity, 0, -1, 4, "0.5"])
    expect(() => setTransition(p, "left", "right", "fade", seconds)).toThrow();
  expect(() => setTransition(p, "left", "right", "movie=foo", 0.5)).toThrow();
  expect(() =>
    applyOperations(p, 1, [
      {
        op: "set_transition",
        fromClipId: "left",
        id: "right",
        kind: "fade",
        duration: 0.5,
      },
    ]),
  ).toThrow("工程已变化");
  for (const patch of [
    { start: 3.2 },
    { start: 2.8 },
    { scale: 0.8 },
    { opacity: 0.5 },
    { trackId: "v2" },
  ]) {
    const invalid = {
      ...p,
      clips: p.clips.map((c) => (c.id === "right" ? { ...c, ...patch } : c)),
    };
    expect(() =>
      setTransition(invalid, "left", "right", "fade", 0.5),
    ).toThrow();
  }
});

test("two cuts from the same source retain independent Agent targets", () => {
  const p = fixture();
  const a = attachmentInput(p, { kind: "clip", id: "left" })!;
  const b = attachmentInput(p, { kind: "clip", id: "right" })!;
  expect(sameInput(a, b)).toBe(false);
  expect(sameInput(a, a)).toBe(true);
  expect(a.key).not.toBe(b.key);
  expect(a.start).toBe(0);
  expect(b.start).toBe(3);
  expect(
    taskAttachments(p, undefined, [a.sourceRef!, b.sourceRef!]).map(
      (a) => a.id,
    ),
  ).toEqual(["left", "right"]);
});
