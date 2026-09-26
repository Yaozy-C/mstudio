import { expect, test } from "bun:test";
import { moveCaption } from "./CaptionTrack";
const c = { id: "caption", text: "字幕", start: 2, end: 5, color: "#ff0000" };
test("moving subtitles preserves duration and styles, clamps at zero and snaps to frames", () => {
  expect(moveCaption(c, -10, 30)).toEqual({ ...c, start: 0, end: 3 });
  expect(moveCaption(c, 0.12, 30).start).toBeCloseTo(2 + 4 / 30);
});
test("subtitle edges cannot cross and preserve the opposite edge", () => {
  expect(moveCaption(c, 10, 30, "left").start).toBeCloseTo(5 - 1 / 30);
  expect(moveCaption(c, -10, 30, "right").end).toBeCloseTo(2 + 1 / 30);
  expect(moveCaption(c, -10, 30, "left").end).toBe(5);
});
