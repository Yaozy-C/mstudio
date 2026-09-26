import { expect, test } from "bun:test";
import {
  newProject,
  makeClip,
  splitClip,
  duration,
  type Asset,
} from "../model";
import { appendAsset, moveClip, addTrack, endTime } from "./document";
import { intervals, inRange, activeAt, upcoming } from "./intervals";
const video: Asset = {
  id: "v",
  name: "video",
  kind: "video",
  path: "",
  preview: "",
  duration: 5,
  width: 160,
  height: 120,
  hasAudio: true,
};
const audio: Asset = { ...video, id: "a", kind: "audio" };
test("explicit clip placement supports overlaps and track moves", () => {
  let p = {
    ...newProject("old"),
    assets: [video, audio],
    clips: [
      makeClip(video),
      { ...makeClip(video), start: 5 },
      { ...makeClip(audio), trackId: "a2" },
    ],
  };
  expect(p.clips.map((c) => c.start)).toEqual([0, 5, 0]);
  expect(endTime(p)).toBe(10);
  p = appendAsset(p, audio, 2, "a1");
  expect(p.clips).toHaveLength(4);
  expect(endTime(p)).toBe(10);
  p = addTrack(p, "video");
  p = moveClip(p, p.clips[1].id, 1, p.tracks!.at(-1)!.id);
  expect(activeAt(intervals(p.clips), 2)).toHaveLength(4);
  const pieces = splitClip(p.clips, p.clips[1].id, 3, 30).filter(
    (c) => c.assetId === "v",
  );
  expect(pieces.map((c) => c.start)).toEqual([0, 1, 3]);
  expect(pieces.reduce((s, c) => s + duration(c), 0)).toBe(10);
  expect(moveClip(p, p.clips[3].id, 0, "v1")).toBe(p);
});
test("overlapping interval tree handles distant long clips and exact half-open boundaries", () => {
  const clips = Array.from({ length: 100000 }, (_, i) => ({
    ...makeClip(video),
    id: `c${i}`,
    start: i,
    trackId: "v1",
    trimOut: 1,
  }));
  clips[0].trimOut = 100000;
  const index = intervals(clips);
  expect(activeAt(index, 90000).map((e) => e.clip.id)).toEqual([
    "c0",
    "c90000",
  ]);
  expect(inRange(index, 90000, 90002)).toHaveLength(4);
  expect(upcoming(index, 90000).map((e) => e.clip.id)).toEqual(["c90001"]);
});
