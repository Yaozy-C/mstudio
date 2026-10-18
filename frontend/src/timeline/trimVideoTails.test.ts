import { expect, test } from "bun:test";
import { duration, makeClip, newProject, type Asset } from "../model";
import { trimVideoTails } from "./trimVideoTails";

const asset: Asset = {
  id: "video",
  kind: "video",
  name: "video",
  path: "video.mp4",
  preview: "",
  duration: 5.184,
  width: 480,
  height: 832,
  hasAudio: true,
};
const ends = { video: 124 / 24 };
function fixture() {
  const p = newProject("test");
  p.assets = [asset];
  p.clips = Array.from({ length: 5 }, (_, i) => ({
    ...makeClip(asset),
    id: String(i),
    start: i * asset.duration,
  }));
  return p;
}

test("five audio-longer clips become 775 contiguous output frames without extending pictures", () => {
  const p = fixture();
  const next = trimVideoTails(p, ends);
  for (const [i, c] of next.clips.entries()) {
    expect(c.start).toBeCloseTo((i * 155) / 30, 12);
    expect(duration(c)).toBeCloseTo(155 / 30, 12);
    expect(c.trimOut).toBeLessThanOrEqual(ends.video);
  }
  expect(trimVideoTails(next, ends)).toBe(next);
  expect(p.clips[0].trimOut).toBe(5.184);
  expect(next.assets).toBe(p.assets);
});

test("respects trim-in, speed, existing shorter edits, intentional gaps and separate audio", () => {
  const p = fixture();
  p.clips = [
    { ...p.clips[0], start: 2, trimIn: 1.2, speed: 2 },
    { ...p.clips[1], start: 8, trimOut: 3 },
    { ...p.clips[2], start: 11, trackId: "a1" },
  ];
  p.captions = [{ id: "c", text: "caption", start: 8, end: 9 }];
  const next = trimVideoTails(p, ends);
  const removed = duration(p.clips[0]) - 59 / 30;
  expect(next.clips[0].trimIn).toBe(1.2);
  expect(next.clips[0].speed).toBe(2);
  expect(duration(next.clips[0])).toBeCloseTo(59 / 30, 12);
  expect(next.clips[1].trimOut).toBe(3);
  expect(next.clips[1].start).toBeCloseTo(8 - removed, 12);
  expect(next.clips[2]).toBe(p.clips[2]);
  expect(next.captions).toBe(p.captions);
});

test("does not trim stills or unknown endpoints, nor alter the project on failure", () => {
  const p = fixture();
  expect(trimVideoTails(p, {})).toBe(p);
  expect(trimVideoTails(p, { video: NaN })).toBe(p);
  const still = { ...asset, id: "still", kind: "image" as const };
  p.assets.push(still);
  p.clips = [{ ...makeClip(still), id: "still", trimOut: 6 }];
  expect(trimVideoTails(p, { still: 5 })).toBe(p);
  const overlap = fixture();
  overlap.clips[1].start = 5.17;
  expect(() => trimVideoTails(overlap, ends)).toThrow("重叠");
  expect(overlap.clips[0].trimOut).toBe(5.184);
  const tiny = fixture();
  tiny.clips[0].trimIn = ends.video - 0.001;
  expect(() => trimVideoTails(tiny, ends)).toThrow("不足一帧");
});

test("retains an existing transition when its shortened seam remains valid", () => {
  const p = fixture();
  const transition = { fromClipId: "0", kind: "fade" as const, duration: 0.3 };
  p.clips[1].transition = transition;
  expect(trimVideoTails(p, ends).clips[1].transition).toBe(transition);
});
