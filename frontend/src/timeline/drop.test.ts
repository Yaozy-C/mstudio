import { expect, test } from "bun:test";
import { duration, makeClip, newProject, type Asset } from "../model";
import { appendAsset } from "./document";
import { CLIP_DRAG_TYPE, REFERENCE_DRAG_TYPE, dropOnTimeline } from "./drop";

const image: Asset = {
  id: "canvas-image",
  name: "画布图片",
  kind: "image",
  path: "/image.png",
  preview: "/image.png",
  duration: 0,
  width: 1080,
  height: 1920,
  hasAudio: false,
};
const reference = JSON.stringify({ kind: "asset", id: image.id });
const project = () => {
  const p = newProject("test");
  return {
    ...p,
    assets: [image],
    tracks: [...p.tracks, { id: "v2", name: "画面 2", kind: "video" as const }],
  };
};

test("canvas image drops create a three-second clip on the target track at a frame-aligned position", () => {
  const p = project();
  const next = dropOnTimeline(p, REFERENCE_DRAG_TYPE, reference, 1.017, "v2");
  expect(next.clips).toHaveLength(1);
  expect(next.clips[0]).toMatchObject({
    assetId: image.id,
    trackId: "v2",
    start: 31 / 30,
  });
  expect(duration(next.clips[0])).toBe(3);
  expect(p.clips).toHaveLength(0);
  expect(next.assets).toBe(p.assets);
  expect(
    dropOnTimeline(p, REFERENCE_DRAG_TYPE, reference, -1, "v1").clips[0].start,
  ).toBe(0);
});

test("menu insertion appends images after existing clips", () => {
  const p = appendAsset(appendAsset(project(), image), image);
  expect(p.clips.map((c) => c.start)).toEqual([0, 3]);
  expect(p.clips[0].id).not.toBe(p.clips[1].id);
});

test("invalid references, missing assets and incompatible tracks leave the timeline unchanged", () => {
  const p = project();
  for (const raw of [
    "{",
    "null",
    "{}",
    JSON.stringify({ kind: "node", id: image.id }),
    JSON.stringify({ kind: "asset", id: "unknown" }),
  ]) {
    expect(dropOnTimeline(p, REFERENCE_DRAG_TYPE, raw, 1, "v1")).toBe(p);
  }
  for (const track of ["a1", "unknown"]) {
    expect(dropOnTimeline(p, REFERENCE_DRAG_TYPE, reference, 1, track)).toBe(p);
  }
  for (const asset of [
    { ...image, missing: true },
    { ...image, kind: "document" as const },
  ]) {
    const invalid = { ...p, assets: [asset] };
    expect(
      dropOnTimeline(invalid, REFERENCE_DRAG_TYPE, reference, 1, "v1"),
    ).toBe(invalid);
  }
});

test("existing clip drops preserve the grab offset and move instead of copying", () => {
  const p = { ...project(), clips: [makeClip(image)] };
  const next = dropOnTimeline(
    p,
    CLIP_DRAG_TYPE,
    JSON.stringify({ id: p.clips[0].id, offset: 0.5 }),
    2,
    "v2",
  );
  expect(next.clips).toHaveLength(1);
  expect(next.clips[0]).toMatchObject({
    id: p.clips[0].id,
    start: 1.5,
    trackId: "v2",
  });
  expect(
    dropOnTimeline(
      p,
      CLIP_DRAG_TYPE,
      JSON.stringify({ id: p.clips[0].id }),
      2,
      "v2",
    ),
  ).toBe(p);
});
