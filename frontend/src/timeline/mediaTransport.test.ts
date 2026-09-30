import { test, expect } from "bun:test";
import { bindVideo } from "./mediaTransport";
import { PlaybackClock } from "./clock";
import type { IndexedClip } from "./geometry";
class FakeVideo extends EventTarget {
  readyState = 4;
  seeking = false;
  ended = false;
  paused = true;
  time = 0;
  rate = 1;
  level = 1;
  seeks = 0;
  rates = 0;
  volumes = 0;
  plays = 0;
  get currentTime() {
    return this.time;
  }
  set currentTime(v: number) {
    this.time = v;
    this.seeks++;
  }
  get playbackRate() {
    return this.rate;
  }
  set playbackRate(v: number) {
    this.rate = v;
    this.rates++;
  }
  get volume() {
    return this.level;
  }
  set volume(v: number) {
    this.level = v;
    this.volumes++;
  }
  async play() {
    this.paused = false;
    this.plays++;
  }
  pause() {
    this.paused = true;
  }
}
const entry: IndexedClip = {
  start: 0,
  end: 4,
  clip: {
    id: "clip",
    assetId: "video",
    trimIn: 2,
    trimOut: 8,
    speed: 1.5,
    volume: 0.8,
    start: 0,
    trackId: "v1",
  },
};
test("normal playback does not repeatedly seek or reset speed; buffering does not run ahead", () => {
  const oldRAF = globalThis.requestAnimationFrame,
    oldCancel = globalThis.cancelAnimationFrame;
  let tick: FrameRequestCallback = () => {};
  globalThis.requestAnimationFrame = (fn) => {
    tick = fn;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(4, 30);
    const video = new FakeVideo();
    const stats = { seeks: 0, waiting: 0, playCalls: 0 };
    const dispose = bindVideo(
      video as unknown as HTMLVideoElement,
      entry,
      clock,
      () => {},
      stats,
    );
    expect(video.currentTime).toBe(2);
    clock.play();
    const now = performance.now();
    for (let i = 1; i <= 120; i++) {
      video.time = 2 + (i / 60) * 1.5;
      tick(now + (i * 1000) / 60);
    }
    expect(clock.getSnapshot().time).toBeCloseTo(2);
    expect(video.seeks).toBe(1);
    expect(video.rates).toBe(1);
    expect(video.volumes).toBe(1);
    expect(video.plays).toBe(1);
    video.dispatchEvent(new Event("waiting"));
    expect(clock.buffering).toBe(true);
    for (let i = 121; i <= 180; i++) tick(now + (i * 1000) / 60);
    expect(clock.getSnapshot().time).toBeCloseTo(2);
    expect(video.seeks).toBe(1);
    video.dispatchEvent(new Event("playing"));
    expect(clock.buffering).toBe(false);
    clock.pause();
    clock.seek(1);
    expect(video.time).toBe(3.5);
    expect(video.seeks).toBe(2);
    clock.step(1);
    expect(video.time).toBeCloseTo(3.55);
    dispose();
    clock.dispose();
  } finally {
    globalThis.requestAnimationFrame = oldRAF;
    globalThis.cancelAnimationFrame = oldCancel;
  }
});
test("secondary tracks follow a single master and pause during its buffering", () => {
  const oldRAF = globalThis.requestAnimationFrame,
    oldCancel = globalThis.cancelAnimationFrame;
  let tick: FrameRequestCallback = () => {};
  globalThis.requestAnimationFrame = (fn) => {
    tick = fn;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(4, 30);
    const master = new FakeVideo(),
      secondary = new FakeVideo();
    const stats = { seeks: 0, waiting: 0, playCalls: 0 };
    const a = bindVideo(
      master as unknown as HTMLVideoElement,
      entry,
      clock,
      () => {},
      stats,
    );
    const b = bindVideo(
      secondary as unknown as HTMLVideoElement,
      {
        ...entry,
        clip: { ...entry.clip, speed: 1, trimIn: 0, trimOut: 4, fadeIn: 1 },
      },
      clock,
      () => {},
      stats,
      false,
    );
    clock.play();
    master.time = 3.5;
    secondary.time = 1;
    tick(performance.now() + 1000);
    expect(clock.getSnapshot().time).toBe(1);
    expect(secondary.volume).toBeCloseTo(0.8);
    master.dispatchEvent(new Event("waiting"));
    expect(secondary.paused).toBe(true);
    master.dispatchEvent(new Event("playing"));
    expect(secondary.paused).toBe(false);
    clock.seek(2);
    expect(master.currentTime).toBe(5);
    expect(secondary.currentTime).toBe(2);
    b();
    master.time = 6.5;
    tick(performance.now() + 2000);
    expect(clock.getSnapshot().time).toBe(3);
    a();
    clock.dispose();
  } finally {
    globalThis.requestAnimationFrame = oldRAF;
    globalThis.cancelAnimationFrame = oldCancel;
  }
});

test("secondary drift uses rate correction until a large discontinuity", () => {
  const oldRAF = globalThis.requestAnimationFrame,
    oldCancel = globalThis.cancelAnimationFrame,
    oldNow = performance.now;
  let tick: FrameRequestCallback = () => {};
  let now = 2000;
  performance.now = () => now;
  globalThis.requestAnimationFrame = (fn) => {
    tick = fn;
    return 1;
  };
  globalThis.cancelAnimationFrame = () => {};
  try {
    const clock = new PlaybackClock();
    clock.configure(4, 30);
    let sourceTime = 0;
    clock.setSource(() => sourceTime);
    const video = new FakeVideo();
    const stats = { seeks: 0, waiting: 0, playCalls: 0 };
    const dispose = bindVideo(
      video as unknown as HTMLVideoElement,
      { ...entry, clip: { ...entry.clip, speed: 1, trimIn: 0, trimOut: 4 } },
      clock,
      () => {},
      stats,
      false,
    );
    clock.play();
    for (let i = 1; i <= 2; i++) {
      now += 1100;
      sourceTime = i;
      video.time = i - 0.2;
      tick(now);
      expect(video.playbackRate).toBeGreaterThan(1);
      expect(video.seeks).toBe(0);
    }
    now += 1100;
    sourceTime = 3;
    video.time = 2.98;
    tick(now);
    expect(video.playbackRate).toBe(1);
    clock.seek(1);
    expect(video.time).toBe(1);
    expect(video.seeks).toBe(1);
    now += 1100;
    sourceTime = 2;
    video.time = 1.2;
    tick(now);
    expect(video.time).toBe(2);
    expect(video.seeks).toBe(2);
    clock.pause();
    expect(video.playbackRate).toBe(1);
    dispose();
    clock.dispose();
  } finally {
    performance.now = oldNow;
    globalThis.requestAnimationFrame = oldRAF;
    globalThis.cancelAnimationFrame = oldCancel;
  }
});

test("preview rate multiplies clip speed without changing source position or clip", () => {
  const clock = new PlaybackClock();
  clock.configure(4, 30);
  const video = new FakeVideo();
  const dispose = bindVideo(
    video as unknown as HTMLVideoElement,
    entry,
    clock,
    () => {},
    { seeks: 0, waiting: 0, playCalls: 0 },
  );
  clock.setRate(0.5);
  expect(video.playbackRate).toBe(0.75);
  expect(video.currentTime).toBe(2);
  clock.seek(2);
  expect(video.currentTime).toBe(5);
  expect(entry.clip.speed).toBe(1.5);
  clock.setRate(2);
  expect(video.playbackRate).toBe(3);
  dispose();
});
