import { test, expect } from "bun:test";
import { decodePreviewFrame } from "./previewFrames";
test("binary frame validates dimensions and preserves RGBA", () => {
  const b = new ArrayBuffer(24),
    h = new DataView(b);
  [7, 2, 1, 10].forEach((v, i) => h.setUint32(i * 4, v, true));
  new Uint8Array(b, 16).set([255, 0, 0, 255, 0, 0, 255, 100]);
  const f = decodePreviewFrame(b)!;
  expect(f.sequence).toBe(7);
  expect([...f.pixels]).toEqual([255, 0, 0, 255, 0, 0, 255, 100]);
  expect(decodePreviewFrame(new ArrayBuffer(0))).toBeNull();
  expect(() => decodePreviewFrame(b.slice(0, 20))).toThrow();
  h.setUint32(4, 999999, true);
  expect(() => decodePreviewFrame(b)).toThrow();
});
