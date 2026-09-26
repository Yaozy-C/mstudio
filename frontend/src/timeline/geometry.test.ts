import { test, expect } from "bun:test";
import {
  clipIndex,
  visibleClips,
  ticks,
  clampZoom,
  frameTime,
} from "./geometry";
import { makeClip, splitClip, type Asset } from "../model";
const asset = { id: "test", kind: "video", duration: 10 } as Asset;
test("100,000 clips render only visible time window and find exact boundary", () => {
  const clip = makeClip(asset);
  const index = clipIndex(
    Array.from({ length: 100000 }, (_, i) => ({
      ...clip,
      id: String(i),
      start: i * 10,
    })),
  );
  expect(visibleClips(index, 500001, 500021).length).toBe(3);
  expect(ticks(500000 * 960, 1200, 960, 30).length).toBeLessThan(25);
});
test("frame precision at 30 and 60fps preserves source trims when splitting", () => {
  for (const fps of [30, 60]) {
    const c = { ...makeClip(asset), speed: 2 };
    const result = splitClip([c], c.id, 1 / fps, fps);
    expect(result.length).toBe(2);
    expect(result[0].trimOut).toBeCloseTo(2 / fps);
    expect(result[1].trimIn).toBe(result[0].trimOut);
    expect(frameTime(1 / fps, fps)).toBe("00:00:01");
    expect(clampZoom(100000, fps) / fps).toBe(32);
  }
});
