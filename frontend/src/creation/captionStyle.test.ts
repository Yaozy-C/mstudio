import { captionPresets, captionKey } from "./captionStyle";
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
    translate: () => {},
    scale: () => {},
    beginPath: () => {},
    roundRect: (...args: unknown[]) => calls.push(["roundRect", ...args]),
    fill: () => calls.push(["fill"]),
    strokeText: (...args: unknown[]) => calls.push(["stroke", ...args]),
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
    captionImage(
      { ...c, strokeWidth: 0.08, backgroundRadius: 0.3 },
      1000,
      1000,
    );
    expect(calls.some((call) => call[0] === "roundRect")).toBe(true);
    expect(calls.some((call) => call[0] === "stroke")).toBe(true);
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
    let animationStored = false;
    const animated = await prepareCaptions(
      [{ ...c, animation: "pop" }],
      1000,
      1000,
      async () => {
        throw new Error("Animation cannot use a static asset");
      },
      {
        fps: 30,
        store: async (frames, duration, loop) => {
          animationStored = true;
          expect(frames.length).toBeGreaterThan(10);
          expect(duration).toBe(1);
          expect(loop).toBe(false);
          return "animated-video";
        },
      },
    );
    expect(animationStored).toBe(true);
    expect(animated[0].assetId).toBe("animated-video");
  } finally {
    globalThis.document = old;
  }
});

test("all presets validate and visual changes invalidate raster cache without timing changes", () => {
  const c: Caption = { id: "sample", start: 0, end: 2, text: "中文字幕" };
  for (const preset of captionPresets)
    expect(validCaption({ ...c, ...preset.style })).toBe(true);
  for (const fields of [
    { strokeWidth: 0.07 },
    { shadowBlur: 0.3 },
    { shadowOffset: 0.1 },
    { backgroundRadius: 0.5 },
    { backgroundColor: "#ff0000" },
    { fontWeight: 900 },
  ]) {
    expect(captionKey({ ...c, ...fields })).not.toBe(captionKey(c));
  }
  expect(captionKey({ ...c, start: 10, end: 12 })).toBe(captionKey(c));
  expect(validCaption({ ...c, strokeWidth: NaN })).toBe(false);
  expect(validCaption({ ...c, shadowColor: "red" })).toBe(false);
  expect(validCaption({ ...c, backgroundOpacity: 2 })).toBe(false);
});
