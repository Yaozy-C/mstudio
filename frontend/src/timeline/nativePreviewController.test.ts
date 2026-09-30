import { test, expect } from "bun:test";
import { PlaybackClock } from "./clock";
import {
  NativePreviewController,
  type PreviewStatus,
} from "./nativePreviewController";
import type { PreviewCommand } from "./previewCommands";
const status = (frame = 0): PreviewStatus => ({
  frame,
  playing: false,
  total: 300,
  rate: 1,
  phase: "paused",
  error: null,
});
const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
};
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
function setup(open = Promise.resolve()) {
  const clock = new PlaybackClock();
  clock.configure(10, 30);
  const calls: (PreviewCommand | { action: "close" })[] = [];
  const phases: string[] = [];
  const backend = {
    open: () => open,
    control: async (command: PreviewCommand | { action: "close" }) => {
      calls.push(command);
      return status(command.action === "seek" ? command.frame : 0);
    },
    status: async () => status(),
  };
  const oldCancel = globalThis.cancelAnimationFrame;
  globalThis.cancelAnimationFrame = () => {};
  const controller = new NativePreviewController(clock, 30, backend, (phase) =>
    phases.push(phase),
  );
  return {
    clock,
    calls,
    phases,
    backend,
    controller,
    cleanup: () => {
      controller.dispose();
      globalThis.cancelAnimationFrame = oldCancel;
    },
  };
}
test("loading keeps latest seek and rate; readiness waits for a presented frame", async () => {
  const opening = deferred<void>();
  const s = setup(opening.promise);
  try {
    const start = s.controller.start();
    s.clock.seek(2);
    s.clock.seek(4);
    s.clock.setRate(1.75);
    expect(s.calls).toEqual([]);
    expect(s.clock.ready).toBe(false);
    opening.resolve();
    await start;
    await flush();
    expect(s.calls).toEqual([
      { action: "rate", rate: 1.75 },
      { action: "seek", frame: 120 },
    ]);
    expect(s.controller.acceptFrame(119)).toBe(false);
    expect(s.controller.acceptFrame(120)).toBe(true);
    expect(s.clock.ready).toBe(false);
    s.controller.presented();
    expect(s.clock.ready).toBe(true);
  } finally {
    s.cleanup();
  }
});
test("late open cannot restart a disposed session", async () => {
  const opening = deferred<void>();
  const s = setup(opening.promise);
  try {
    const start = s.controller.start();
    s.controller.dispose();
    opening.resolve();
    await start;
    expect(s.calls).toEqual([{ action: "close" }]);
    expect(s.controller.phase).toBe("closed");
    expect(s.phases).toEqual(["loading"]);
  } finally {
    s.cleanup();
  }
});
test("old status response cannot overwrite a newer seek", async () => {
  const s = setup();
  const pending = deferred<PreviewStatus>();
  try {
    await s.controller.start();
    await flush();
    s.backend.status = () => pending.promise;
    const poll = s.controller.poll();
    s.clock.seek(5);
    await flush();
    pending.resolve(status(10));
    await poll;
    expect(s.clock.getSnapshot().time).toBe(5);
  } finally {
    s.cleanup();
  }
});
test("worker failure stops keyboard playback and requires a fresh session", async () => {
  const s = setup();
  try {
    await s.controller.start();
    await flush();
    s.controller.presented();
    s.backend.status = async () => {
      throw Error("worker exited");
    };
    await s.controller.poll();
    expect(s.controller.phase).toBe("error");
    expect(s.clock.ready).toBe(false);
    s.clock.play();
    expect(s.clock.getSnapshot().playing).toBe(false);
    expect(s.calls.at(-1)).toEqual({ action: "close" });
    s.controller.presented();
    expect(s.clock.ready).toBe(false);
  } finally {
    s.cleanup();
  }
});
test("late failure from retired session cannot disable its replacement", async () => {
  const s = setup();
  const pending = deferred<PreviewStatus>();
  try {
    await s.controller.start();
    await flush();
    s.backend.status = () => pending.promise;
    const poll = s.controller.poll();
    s.controller.dispose();
    s.clock.ready = true;
    pending.reject(Error("old worker exited"));
    await poll;
    expect(s.clock.ready).toBe(true);
    expect(s.phases).not.toContain("error");
  } finally {
    s.cleanup();
  }
});
