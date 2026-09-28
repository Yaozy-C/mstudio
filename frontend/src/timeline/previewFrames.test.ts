import { test, expect } from "bun:test";
import { decodePreviewFrame, startPreviewFrames } from "./previewFrames";
test("binary frame validates dimensions and preserves RGBA", () => {
  const b = new ArrayBuffer(24),
    h = new DataView(b);
  [7, 2, 1, 10].forEach((v, i) => h.setUint32(i * 4, v, true));
  new Uint8Array(b, 16).set([255, 0, 0, 255, 0, 0, 255, 100]);
  const f = decodePreviewFrame(b)!;
  expect(f.sequence).toBe(7);
  expect(f.position).toBe(10);
  expect([...f.pixels]).toEqual([255, 0, 0, 255, 0, 0, 255, 100]);
  expect(decodePreviewFrame(new ArrayBuffer(0))).toBeNull();
  expect(() => decodePreviewFrame(b.slice(0, 20))).toThrow();
  h.setUint32(4, 999999, true);
  expect(() => decodePreviewFrame(b)).toThrow();
});

test("preroll and stale seek replies never paint or reveal the canvas", async () => {
  const names = [
    "requestAnimationFrame",
    "cancelAnimationFrame",
    "document",
    "ImageData",
  ] as const;
  const originals = names.map((name) =>
    Object.getOwnPropertyDescriptor(globalThis, name),
  );
  let tick: FrameRequestCallback;
  let resolve!: (value: ArrayBuffer) => void;
  let busy = true,
    target = 380,
    painted = 0,
    ready = 0;
  const sequences: number[] = [];
  const packet = (sequence: number, position: number) => {
    const b = new ArrayBuffer(20),
      h = new DataView(b);
    [sequence, 1, 1, position].forEach((v, i) => h.setUint32(i * 4, v, true));
    return b;
  };
  const set = (name: string, value: unknown) =>
    Object.defineProperty(globalThis, name, { configurable: true, value });
  try {
    set("requestAnimationFrame", (fn: FrameRequestCallback) => {
      tick = fn;
      return 1;
    });
    set("cancelAnimationFrame", () => {});
    set("document", { hidden: false });
    set("ImageData", class {});
    const stop = startPreviewFrames(
      {
        width: 1,
        height: 1,
        getContext: () => ({ putImageData: () => painted++ }),
      } as unknown as HTMLCanvasElement,
      (last) => {
        sequences.push(last);
        return new Promise((r) => {
          resolve = r;
        });
      },
      (error) => {
        throw error;
      },
      (position) => !busy && position === target,
      () => ready++,
    );
    const deliver = async (sequence: number, position: number) => {
      tick(0);
      resolve(packet(sequence, position));
      await Promise.resolve();
    };
    await deliver(1, 0);
    busy = false;
    await deliver(2, 0);
    expect(painted).toBe(0);
    expect(ready).toBe(0);
    await deliver(3, 380);
    expect(painted).toBe(1);
    expect(ready).toBe(1);
    tick!(0);
    target = 389;
    busy = true;
    resolve(packet(4, 380));
    await Promise.resolve();
    expect(painted).toBe(1);
    busy = false;
    await deliver(5, 389);
    expect(painted).toBe(2);
    expect(ready).toBe(1);
    expect(sequences).toEqual([0, 0, 0, 3, 3]);
    tick!(0);
    stop();
    resolve(packet(6, 389));
    await Promise.resolve();
    expect(painted).toBe(2);
  } finally {
    names.forEach((name, i) => {
      if (originals[i]) Object.defineProperty(globalThis, name, originals[i]!);
      else Reflect.deleteProperty(globalThis, name);
    });
  }
});
