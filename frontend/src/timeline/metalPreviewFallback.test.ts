import { expect, test } from "bun:test";
import { startMetalPreviewFrames } from "./metalPreviewFrames";

test("overlapping menus use live canvas frames and return to Metal when closed", async () => {
  const keys = [
    "document",
    "devicePixelRatio",
    "requestAnimationFrame",
    "cancelAnimationFrame",
    "ImageData",
  ];
  const originals = keys.map((key) =>
    Object.getOwnPropertyDescriptor(globalThis, key),
  );
  let tick!: () => Promise<void>;
  let hidden = false,
    occluded = true,
    busy = false;
  let target = 42,
    ready = 0,
    nativePaints = 0;
  let resolve!: (buffer: ArrayBuffer) => void;
  const reads: number[] = [],
    events: string[] = [];
  const canvas = {
    width: 0,
    height: 0,
    getContext: () => ({ putImageData: () => events.push("canvas") }),
  } as unknown as HTMLCanvasElement;
  const packet = (sequence: number, position: number) => {
    const data = new ArrayBuffer(20);
    [sequence, 1, 1, position].forEach((value, i) =>
      new DataView(data).setUint32(i * 4, value, true),
    );
    return data;
  };
  try {
    const globals = [
      {
        get hidden() {
          return hidden;
        },
        addEventListener: () => {},
        removeEventListener: () => {},
      },
      1,
      (fn: () => Promise<void>) => {
        tick = fn;
        return 1;
      },
      () => {},
      class {},
    ];
    globals.forEach((value, i) =>
      Object.defineProperty(globalThis, keys[i], {
        configurable: true,
        value,
      }),
    );
    const stop = startMetalPreviewFrames(
      () => ({
        x: 0,
        y: 0,
        width: 160,
        height: 90,
        viewportWidth: 800,
        viewportHeight: 600,
        visible: !occluded && !hidden,
      }),
      {
        layout: async (bounds) => {
          events.push(`metal:${bounds.visible}`);
        },
        read: async () => ({ sequence: 8, position: target, generation: 1 }),
        present: async () => {
          nativePaints++;
          return true;
        },
      },
      (error) => {
        throw error;
      },
      (position) => !busy && position === target,
      () => ready++,
      {
        canvas,
        read: (last) => {
          reads.push(last);
          return new Promise((reply) => {
            resolve = reply;
          });
        },
      },
    );
    const deliver = async (
      sequence: number,
      position: number,
      beforeReply = () => {},
    ) => {
      const pending = tick();
      beforeReply();
      resolve(packet(sequence, position));
      await pending;
    };
    await deliver(7, 0);
    expect(events).toEqual(["metal:false"]);
    expect(ready).toBe(0);
    // Remove and reopen the menu: paint before hiding the native surface.
    occluded = false;
    await tick();
    occluded = true;
    events.length = 0;
    await deliver(7, 42);
    expect(events).toEqual(["canvas", "metal:false"]);
    expect([canvas.width, canvas.height]).toEqual([1, 1]);
    expect(ready).toBe(1);
    target = 43;
    await deliver(8, 42);
    await deliver(9, 43, () => {
      busy = true;
    });
    expect(events.filter((event) => event === "canvas")).toHaveLength(1);
    busy = false;
    await deliver(10, 43);
    expect(events.filter((event) => event === "canvas")).toHaveLength(2);
    expect(reads).toEqual([0, 0, 7, 7, 7]);
    occluded = false;
    await tick();
    expect(nativePaints).toBe(2);
    expect(events.at(-1)).toBe("metal:true");
    hidden = true;
    await tick();
    expect(reads).toHaveLength(5);
    hidden = false;
    occluded = true;
    await deliver(11, 43, stop);
    expect(events.filter((event) => event === "canvas")).toHaveLength(2);
  } finally {
    keys.forEach((key, i) => {
      if (originals[i]) Object.defineProperty(globalThis, key, originals[i]!);
      else Reflect.deleteProperty(globalThis, key);
    });
  }
});
