import { expect, test } from "bun:test";
import {
  metalBounds,
  startMetalPreviewFrames,
  type MetalBounds,
  type MetalFrame,
} from "./metalPreviewFrames";

test("Metal fits the video inside the border and hides beneath intersecting DOM overlays", () => {
  const originals = ["document", "innerWidth", "innerHeight"].map((key) =>
    Object.getOwnPropertyDescriptor(globalThis, key),
  );
  const keys = ["document", "innerWidth", "innerHeight"];
  let overlays: unknown[] = [],
    hidden = false;
  const stage = {
    isConnected: true,
    contains: () => true,
    getBoundingClientRect: () => ({
      left: 20,
      top: 100,
      width: 802,
      height: 602,
    }),
  } as unknown as HTMLElement;
  try {
    const doc = {
      get hidden() {
        return hidden;
      },
      elementFromPoint: () => stage,
      querySelectorAll: () => overlays,
    };
    [doc, 1600, 1000].forEach((value, i) =>
      Object.defineProperty(globalThis, keys[i], { configurable: true, value }),
    );
    expect(metalBounds(stage, 1920, 1080)).toEqual({
      x: 21,
      y: 176,
      width: 800,
      height: 450,
      viewportWidth: 1600,
      viewportHeight: 1000,
      visible: true,
    });
    overlays = [
      {
        getBoundingClientRect: () => ({
          left: 10,
          right: 300,
          top: 150,
          bottom: 250,
          width: 290,
          height: 100,
        }),
      },
    ];
    expect(metalBounds(stage, 1920, 1080).visible).toBe(false);
    overlays = [
      {
        getBoundingClientRect: () => ({
          left: 1000,
          right: 1100,
          top: 0,
          bottom: 200,
          width: 100,
          height: 200,
        }),
      },
    ];
    expect(metalBounds(stage, 1920, 1080).visible).toBe(true);
    hidden = true;
    expect(metalBounds(stage, 1920, 1080).visible).toBe(false);
  } finally {
    keys.forEach((key, i) => {
      if (originals[i]) Object.defineProperty(globalThis, key, originals[i]!);
      else Reflect.deleteProperty(globalThis, key);
    });
  }
});

test("Metal transport rejects stale seeks, retries GPU backpressure and redraws after visibility or DPI changes", async () => {
  const keys = [
    "document",
    "devicePixelRatio",
    "requestAnimationFrame",
    "cancelAnimationFrame",
  ];
  const originals = keys.map((key) =>
    Object.getOwnPropertyDescriptor(globalThis, key),
  );
  let tick!: () => Promise<void>,
    changed!: () => void,
    hidden = false,
    busy = true,
    ready = 0,
    paints = 0,
    allow = false;
  const layouts: boolean[] = [],
    reads: number[] = [];
  let deliver!: (frame: MetalFrame | null) => void;
  const bounds = (): MetalBounds => ({
    x: 0,
    y: 0,
    width: 160,
    height: 90,
    viewportWidth: 800,
    viewportHeight: 600,
    visible: !hidden,
  });
  try {
    const doc = {
      get hidden() {
        return hidden;
      },
      addEventListener: (_: string, fn: () => void) => {
        changed = fn;
      },
      removeEventListener: () => {},
    };
    const globals = [
      doc,
      2,
      (fn: () => Promise<void>) => {
        tick = fn;
        return 1;
      },
      () => {},
    ];
    globals.forEach((value, i) =>
      Object.defineProperty(globalThis, keys[i], {
        configurable: true,
        value,
        writable: true,
      }),
    );
    const stop = startMetalPreviewFrames(
      bounds,
      {
        layout: async (b) => {
          layouts.push(b.visible);
        },
        read: (last) => {
          reads.push(last);
          return new Promise((resolve) => {
            deliver = resolve;
          });
        },
        present: async () => {
          paints++;
          return allow;
        },
      },
      (error) => {
        throw error;
      },
      (position) => !busy && position === 42,
      () => ready++,
    );
    const frame = { sequence: 7, generation: 3, position: 42 };
    const run = async (value: MetalFrame | null, beforeReply = () => {}) => {
      const pending = tick();
      await Promise.resolve();
      beforeReply();
      deliver(value);
      await pending;
    };
    await run(frame);
    expect(paints).toBe(0);
    busy = false;
    await run({ ...frame, position: 0 });
    expect(paints).toBe(0);
    await run(frame);
    expect(ready).toBe(0);
    allow = true;
    await run(frame);
    expect(ready).toBe(1);
    await run(null);
    expect(reads.at(-1)).toBe(7);
    hidden = true;
    changed();
    expect(layouts.at(-1)).toBe(false);
    hidden = false;
    changed();
    await run(frame);
    expect(layouts.at(-1)).toBe(true);
    expect(reads.at(-1)).toBe(0);
    Object.defineProperty(globalThis, "devicePixelRatio", { value: 1 });
    await run(frame);
    expect(reads.at(-1)).toBe(0);
    const before = paints;
    await run(frame, stop);
    expect(paints).toBe(before);
    expect(layouts.at(-1)).toBe(false);
  } finally {
    keys.forEach((key, i) => {
      if (originals[i]) Object.defineProperty(globalThis, key, originals[i]!);
      else Reflect.deleteProperty(globalThis, key);
    });
  }
});
