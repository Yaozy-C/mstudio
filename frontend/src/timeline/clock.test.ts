import { test, expect } from "bun:test";
import { PlaybackClock } from "./clock";
test("clock seeks to frames, clamps bounds, and ends playback without touching project state", () => {
  const raf = globalThis.requestAnimationFrame;
  const caf = globalThis.cancelAnimationFrame;
  let callback: FrameRequestCallback = () => {};
  globalThis.requestAnimationFrame = (fn) => {
    callback = fn;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(1, 30);
    clock.seek(0.052);
    expect(clock.getSnapshot().time).toBe(2 / 30);
    clock.step(-1);
    expect(clock.getSnapshot().time).toBe(1 / 30);
    clock.seek(4);
    expect(clock.getSnapshot().time).toBe(1);
    clock.ready = false;
    clock.play();
    expect(clock.getSnapshot().playing).toBe(false);
    clock.ready = true;
    clock.play();
    expect(clock.getSnapshot().time).toBe(0);
    const now = performance.now();
    for (let i = 1; i <= 12; i++) callback(now + i * 100);
    expect(clock.getSnapshot()).toEqual({ time: 1, playing: false });
    clock.dispose();
  } finally {
    globalThis.requestAnimationFrame = raf;
    globalThis.cancelAnimationFrame = caf;
  }
});

test("native transport owns time and replay sends one seek before play", () => {
  const oldRAF = globalThis.requestAnimationFrame,
    oldCancel = globalThis.cancelAnimationFrame;
  let rafs = 0;
  globalThis.requestAnimationFrame = () => {
    rafs++;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(5, 30);
    const calls: string[] = [];
    const detach = clock.attachTransport({
      play: () => calls.push("play"),
      pause: () => calls.push("pause"),
      seek: (t) => calls.push(`seek:${t}`),
    });
    clock.play();
    expect(rafs).toBe(0);
    clock.acceptTransportState(2, true);
    expect(clock.getSnapshot().time).toBe(2);
    clock.pause();
    clock.seek(1.05);
    expect(calls.at(-1)).toBe(`seek:${32 / 30}`);
    clock.acceptTransportState(5, false);
    calls.length = 0;
    clock.play();
    expect(calls).toEqual(["seek:0", "play"]);
    detach();
    clock.acceptTransportState(3, true);
    expect(clock.getSnapshot().playing).toBe(false);
  } finally {
    globalThis.requestAnimationFrame = oldRAF;
    globalThis.cancelAnimationFrame = oldCancel;
  }
});

test("stationary clicks do not repeat native pause or seek", () => {
  const cancel = globalThis.cancelAnimationFrame;
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(5, 30);
    const calls: string[] = [];
    clock.attachTransport({
      play: () => calls.push("play"),
      pause: () => calls.push("pause"),
      seek: (t) => calls.push(`seek:${t}`),
    });
    clock.pause();
    clock.seek(1);
    clock.seek(1.001);
    clock.pause();
    expect(calls).toEqual(["seek:1"]);
    clock.play();
    clock.pause();
    clock.pause();
    expect(calls).toEqual(["seek:1", "play", "pause"]);
  } finally {
    globalThis.cancelAnimationFrame = cancel;
  }
});

test("preview speed accepts quarter steps and changes fallback time independently", () => {
  const raf = globalThis.requestAnimationFrame,
    caf = globalThis.cancelAnimationFrame;
  let tick: FrameRequestCallback = () => {};
  globalThis.requestAnimationFrame = (fn) => {
    tick = fn;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(20, 30);
    for (const rate of [0, 0.3, 2.25, NaN, Infinity])
      expect(() => clock.setRate(rate)).toThrow();
    clock.setRate(2);
    clock.play();
    tick(performance.now() + 500);
    expect(clock.getSnapshot().time).toBeCloseTo(1, 1);
    clock.pause();
    const calls: number[] = [];
    clock.attachTransport({
      play: () => {},
      pause: () => {},
      seek: () => {},
      rate: (r) => calls.push(r),
    });
    clock.setRate(0.75);
    expect(clock.getRate()).toBe(0.75);
    expect(calls).toEqual([0.75]);
    expect(clock.total).toBe(20);
  } finally {
    globalThis.requestAnimationFrame = raf;
    globalThis.cancelAnimationFrame = caf;
  }
});
