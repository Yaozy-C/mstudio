import { expect, test } from "bun:test";
import { validCaption, captionImage } from "./captions";
import { prepareCaptions } from "./prepareCaptions";
import type { Caption } from "../model";

test("caption styles validate and same text with different styles gets distinct raster assets", async () => {
  const c: Caption = {
    id: "a",
    start: 0,
    end: 1,
    text: "style-cache-regression",
    font: "serif",
    color: "#ff0000",
    fontSize: 0.06,
    x: 0.3,
    y: 0.4,
  };
  expect(validCaption(c)).toBe(true);
  expect(validCaption({ ...c, color: "red" })).toBe(false);
  expect(validCaption({ ...c, x: NaN })).toBe(false);
  const old = globalThis.document;
  const calls: unknown[][] = [];
  let count = 0;
  const ctx = {
    font: "",
    fillStyle: "",
    shadowColor: "",
    shadowBlur: 0,
    textAlign: "",
    textBaseline: "",
    measureText: () => ({ width: 20 }),
    fillRect: (...args: unknown[]) => calls.push(["rect", ...args]),
    fillText: (text: string, x: number, y: number) =>
      calls.push([text, x, y, ctx.font, ctx.fillStyle]),
  };
  globalThis.document = {
    createElement: () => ({
      width: 0,
      height: 0,
      getContext: () => ctx,
      toDataURL: () => `image-${++count}`,
    }),
  } as unknown as Document;
  try {
    const image = captionImage(c, 1000, 1000);
    expect(calls.at(-1)).toEqual([
      c.text,
      300,
      359.5,
      expect.stringContaining("Songti SC"),
      "#ff0000",
    ]);
    expect(
      captionImage({ ...c, id: "same", start: 2, end: 3 }, 1000, 1000),
    ).toBe(image);
    const stored: string[] = [];
    const result = await prepareCaptions(
      [c, { ...c, id: "b", color: "#00ff00" }, { ...c, id: "c" }],
      1000,
      1000,
      async (data) => {
        stored.push(data);
        return `asset-${stored.length}`;
      },
    );
    expect(stored.length).toBe(2);
    expect(result.map((c) => c.assetId)).toEqual([
      "asset-1",
      "asset-2",
      "asset-1",
    ]);
  } finally {
    globalThis.document = old;
  }
});
