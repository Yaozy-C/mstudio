import { expect, test } from "bun:test";
import { makeClip, newProject, type Asset } from "../model";
import { detachAudio } from "./detachAudio";
const video: Asset = {
  id: "v",
  kind: "video",
  name: "video",
  path: "/video.mp4",
  preview: "",
  duration: 20,
  width: 1920,
  height: 1080,
  hasAudio: true,
};
test("detached audio preserves timing, speed and gain without doubling the video sound", () => {
  const p = newProject("test");
  p.assets = [video];
  p.tracks[0].muted = true;
  const clip = {
    ...makeClip(video),
    trackId: "v1",
    start: 4,
    trimIn: 2,
    trimOut: 10,
    speed: 2,
    volume: 0.7,
    fadeIn: 0.3,
    fadeOut: 0.4,
  };
  p.clips = [clip];
  const q = detachAudio(p, clip.id);
  expect(q.clips[0]).toEqual({ ...clip, volume: 0 });
  const audio = q.clips[1];
  expect(audio).toEqual({
    ...clip,
    id: audio.id,
    trackId: q.tracks.at(-1)!.id,
  });
  expect(audio.id).not.toBe(clip.id);
  expect(q.tracks.at(-1)!.kind).toBe("audio");
  expect(q.tracks.at(-1)!.muted).toBe(true);
  expect(q.assets).toBe(p.assets);
  expect(p.clips).toEqual([clip]);
  expect(detachAudio(q, audio.id)).toBe(q);
  expect(detachAudio(p, "missing")).toBe(p);
});
test("silent videos cannot be detached", () => {
  const p = newProject("silent");
  p.assets = [{ ...video, hasAudio: false }];
  p.clips = [makeClip(video)];
  expect(detachAudio(p, p.clips[0].id)).toBe(p);
});
