import { endTime } from "./timeline/document";
import { test, expect } from "bun:test";
import { duration, splitClip, newProject, type Clip } from "./model";
const clip: Clip = {
  id: "one",
  assetId: "asset",
  trimIn: 2,
  trimOut: 8,
  speed: 1.5,
  volume: 1,
  start: 0,
  trackId: "v1",
};
test("split uses source coordinates at playback speed and preserves duration", () => {
  const clips = splitClip([clip], "one", 2);
  expect(clips.length).toBe(2);
  expect(clips[0].trimOut).toBe(5);
  expect(clips[1].trimIn).toBe(5);
  expect(clips[1].id).not.toBe(clip.id);
  expect(endTime({ ...newProject("test"), clips })).toBe(duration(clip));
});
test("split after earlier clips accounts for timeline offset", () => {
  const second = { ...clip, id: "two", start: 4, speed: 2 };
  const clips = splitClip([clip, second], "two", 5);
  expect(clips[1].trimOut).toBe(4);
  expect(clips[2].trimIn).toBe(4);
  expect(endTime({ ...newProject("test"), clips })).toBe(7);
});
test("boundary splits do not create zero length clips", () => {
  expect(splitClip([clip], "one", 0)).toEqual([clip]);
  expect(splitClip([clip], "one", 4)).toEqual([clip]);
});
