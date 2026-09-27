import { expect, test } from "bun:test";
import { makeClip, newProject, duration, type Asset } from "../model";
import { applyOperations } from "../assistant/projectCommands";

const asset: Asset = {
  id: "source",
  name: "source",
  kind: "video",
  path: "",
  preview: "",
  duration: 20,
  width: 1920,
  height: 1080,
  hasAudio: true,
};
function fixture() {
  const p = newProject("editing");
  p.assets = [asset];
  p.clips = [
    { ...makeClip(asset), id: "a", trimIn: 2, trimOut: 6 },
    {
      ...makeClip(asset),
      id: "b",
      trimIn: 6,
      trimOut: 10,
      start: 4,
      transition: { fromClipId: "a", kind: "fade", duration: 0.4 },
    },
    { ...makeClip(asset), id: "sound", trackId: "a1", trimOut: 10 },
  ];
  p.captions = [{ id: "caption", start: 4, end: 5, text: "fixed" }];
  return p;
}
test("retime ripples the same track, preserves source range and valid seam, leaves audio and captions", () => {
  const p = fixture();
  const next = applyOperations(p, 0, [
    { op: "retime_clip", id: "a", speed: 2, ripple: true },
  ]);
  expect(duration(next.clips[0])).toBe(2);
  expect(next.clips[0].trimIn).toBe(2);
  expect(next.clips[0].trimOut).toBe(6);
  expect(next.clips[1].start).toBe(2);
  expect(next.clips[1].transition).toEqual(p.clips[1].transition);
  expect(next.clips[2]).toEqual(p.clips[2]);
  expect(next.captions).toEqual(p.captions);
  expect(p.clips[1].start).toBe(4);
});
test("slowdown refuses unintended overlap, ripple closes the edit without collision", () => {
  const p = fixture();
  expect(() =>
    applyOperations(p, 0, [{ op: "retime_clip", id: "a", speed: 0.5 }]),
  ).toThrow("重叠");
  const next = applyOperations(p, 0, [
    { op: "retime_clip", id: "a", speed: 0.5, ripple: true },
  ]);
  expect(next.clips[1].start).toBe(8);
});
test("move snaps to frames and removes disconnected transitions without changing media", () => {
  const p = fixture();
  const next = applyOperations(p, 0, [
    { op: "move_clip", id: "b", start: 9.011 },
  ]);
  expect(next.clips[1].start).toBe(9);
  expect(next.clips[1].transition).toBeUndefined();
  expect(next.clips[1].trimIn).toBe(6);
  expect(next.clips[1].speed).toBe(1);
  expect(() =>
    applyOperations(p, 0, [{ op: "move_clip", id: "b", start: 2 }]),
  ).toThrow("重叠");
});
test("slip uses source seconds, preserves timing and refuses unavailable handles atomically", () => {
  const p = fixture();
  const next = applyOperations(p, 0, [
    { op: "slip_clip", id: "a", sourceOffset: 1.5 },
  ]);
  expect(next.clips[0].trimIn).toBe(3.5);
  expect(next.clips[0].trimOut).toBe(7.5);
  expect(duration(next.clips[0])).toBe(4);
  expect(next.clips[1]).toEqual(p.clips[1]);
  expect(() =>
    applyOperations(p, 0, [{ op: "slip_clip", id: "a", sourceOffset: -3 }]),
  ).toThrow();
  expect(p.clips[0].trimIn).toBe(2);
});
test("reject invalid speed, flags, tracks and ripple across an overlapping tail", () => {
  const p = fixture();
  for (const speed of [0, NaN, Infinity, 5])
    expect(() =>
      applyOperations(p, 0, [{ op: "retime_clip", id: "a", speed }]),
    ).toThrow();
  expect(() =>
    applyOperations(p, 0, [
      { op: "retime_clip", id: "a", speed: 2, ripple: "true" },
    ]),
  ).toThrow();
  expect(() =>
    applyOperations(p, 0, [
      { op: "move_clip", id: "a", start: 0, trackId: "missing" },
    ]),
  ).toThrow();
  p.clips[1].start = 3;
  expect(() =>
    applyOperations(p, 0, [
      { op: "retime_clip", id: "a", speed: 2, ripple: true },
    ]),
  ).toThrow("尾部");
});
