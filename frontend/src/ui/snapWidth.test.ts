import { expect, test } from "bun:test";
import { snapWidth } from "./snapWidth";

test("resizing enters a stop narrowly and releases it beyond the hysteresis band", () => {
  const stops = [180, 240, 320];
  expect(snapWidth(233, stops, null)).toBeNull();
  expect(snapWidth(235, stops, null)).toBe(240);
  expect(snapWidth(250, stops, 240)).toBe(240);
  expect(snapWidth(253, stops, 240)).toBeNull();
  expect(snapWidth(317, stops, null)).toBe(320);
  expect(snapWidth(240, [180], 240)).toBeNull();
});
