import { test, expect } from "bun:test";
import { waveformPath } from "./Waveform";
test("waveform selects source trim range and applies clip gain", () => {
  const peaks = Array.from({ length: 2048 }, (_, i) => (i < 1024 ? 0 : 1));
  const silent = waveformPath(peaks, { trimIn: 0, trimOut: 1, volume: 1 }, 2);
  const loud = waveformPath(peaks, { trimIn: 1, trimOut: 2, volume: 1 }, 2);
  expect(silent).toStartWith("M0,15.7v0.6");
  expect(loud).toStartWith("M0,2v28");
  expect(
    waveformPath(peaks, { trimIn: 1, trimOut: 2, volume: 0.5 }, 2),
  ).toStartWith("M0,9v14");
});
