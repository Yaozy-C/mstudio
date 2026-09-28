import { expect, test } from "bun:test";
import { duration, makeClip, newProject, type Asset } from "../model";
import { packClips } from "./packClips";
import { speedForDuration, targetDurationRange } from "./targetDuration";

const asset: Asset = {
  id: "media",
  name: "media",
  path: "",
  preview: "",
  kind: "video",
  duration: 10,
  width: 100,
  height: 100,
  hasAudio: true,
};

test("packing follows timeline order, removes gaps and overlaps, preserves trims and other tracks", () => {
  const p = newProject("test");
  const a = {
    ...makeClip(asset),
    id: "a",
    start: 2,
    trimIn: 1,
    trimOut: 4,
    speed: 1.3,
  };
  const b = { ...makeClip(asset), id: "b", start: 3, trimOut: 2 };
  const c = {
    ...makeClip(asset),
    id: "c",
    start: 12,
    transition: { fromClipId: "b", kind: "fade" as const, duration: 0.3 },
  };
  const audio = { ...makeClip(asset), id: "audio", trackId: "a1", start: 7 };
  p.clips = [c, audio, a, b];
  p.captions = [{ id: "caption", start: 8, end: 9, text: "caption" }];
  const next = packClips(p, "v1");
  expect(next.clips[2]).toEqual({ ...a, start: 0 });
  expect(next.clips[3]).toEqual({ ...b, start: duration(a) });
  expect(next.clips[0]).toEqual({ ...c, start: duration(a) + duration(b) });
  expect(next.clips[1]).toBe(audio);
  expect(next.captions).toBe(p.captions);
  expect(p.clips[2].start).toBe(2);
  expect(packClips(next, "v1")).toBe(next);
  expect(packClips(p).clips[1].start).toBe(0);
  expect(packClips(p, "missing")).toBe(p);
});

test("target duration uses the trimmed source, independent of existing speed", () => {
  const clip = { ...makeClip(asset), trimIn: 2, trimOut: 5, speed: 1.3 };
  expect(speedForDuration(clip, 1.5)).toBe(2);
  expect(
    duration({ ...clip, speed: speedForDuration(clip, 1.7)! }),
  ).toBeCloseTo(1.7, 12);
  expect(targetDurationRange(clip)).toEqual({ min: 0.8, max: 12 });
  for (const value of [0, -1, NaN, Infinity, 0.7, 12.1, 1.55])
    expect(speedForDuration(clip, value)).toBeNull();
  expect(speedForDuration(clip, 12)).toBe(0.25);
  expect(targetDurationRange({ ...clip, trimOut: 2.01 }).max).toBe(0);
});
