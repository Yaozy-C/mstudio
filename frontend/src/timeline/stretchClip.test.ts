import { expect, test } from "bun:test";
import { stretchClip } from "./stretchClip";
import { trackRows } from "./trackRows";
import { duration, type Clip } from "../model";
const clip: Clip = {
  id: "c",
  assetId: "a",
  trackId: "v",
  start: 3,
  trimIn: 2,
  trimOut: 10,
  speed: 1,
  volume: 1,
};
test("dragging edges changes rate without discarding source frames", () => {
  const right = stretchClip(clip, -4, "right", 30);
  expect(right).toEqual({ ...clip, speed: 2 });
  const left = stretchClip(clip, 4, "left", 30);
  expect(left).toEqual({ ...clip, start: 7, speed: 2 });
  expect(left.start + duration(left)).toBe(11);
  expect(stretchClip(clip, 8, "right", 30).speed).toBe(0.5);
  expect(stretchClip(clip, -100, "right", 30).speed).toBe(4);
  expect(stretchClip(clip, 100, "right", 30).speed).toBe(0.25);
  expect(stretchClip(clip, -100, "left", 30).start).toBe(0);
});
test("separated audio sits below its video while visual stacking stays unchanged", () => {
  const rows = trackRows([
    { id: "v1", kind: "video", name: "1" },
    { id: "a", kind: "audio", name: "a", sourceTrackId: "v1" },
    { id: "other", kind: "audio", name: "other" },
    { id: "v2", kind: "video", name: "2" },
  ]);
  expect(rows.map((t) => t.id)).toEqual(["v2", "v1", "a", "other"]);
  expect(trackRows(rows.filter((t) => t.id !== "v1")).map((t) => t.id)).toEqual(
    ["v2", "a", "other"],
  );
});
