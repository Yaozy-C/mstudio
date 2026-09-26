import { expect, test } from "bun:test";
import { makeClip, newProject, type Asset } from "../model";
import { addTrack, appendAsset, removeTrack } from "./document";
const image: Asset = {
  id: "image",
  name: "Image",
  kind: "image",
  path: "/image.png",
  preview: "",
  duration: 0,
  width: 10,
  height: 10,
  hasAudio: false,
};
test("deleting a track removes only its clips and preserves assets and the undo snapshot", () => {
  const base = addTrack(newProject("test"), "video");
  const id = base.tracks.at(-1)!.id;
  const original = {
    ...base,
    assets: [image],
    clips: [makeClip(image), { ...makeClip(image), trackId: id }],
  };
  const result = removeTrack(original, id);
  expect(result.tracks.some((t) => t.id === id)).toBe(false);
  expect(result.clips).toEqual([original.clips[0]]);
  expect(result.assets).toBe(original.assets);
  expect(result.captions).toBe(original.captions);
  expect(original.clips).toHaveLength(2);
  expect(original.tracks.some((t) => t.id === id)).toBe(true);
  expect(removeTrack(original, "missing")).toBe(original);
});
test("deleting every track still allows inserting visual and audio assets", () => {
  const initial = newProject("test");
  const empty = initial.tracks.reduce((p, t) => removeTrack(p, t.id), initial);
  expect(empty.tracks).toHaveLength(0);
  const visual = appendAsset(empty, image);
  expect(visual.tracks).toHaveLength(1);
  expect(visual.tracks[0].kind).toBe("video");
  expect(visual.clips[0].trackId).toBe(visual.tracks[0].id);
  const audio = appendAsset(visual, {
    ...image,
    id: "audio",
    kind: "audio",
    duration: 5,
    hasAudio: true,
  });
  expect(audio.tracks).toHaveLength(2);
  expect(audio.clips[1].trackId).toBe(audio.tracks[1].id);
  expect(audio.tracks[1].kind).toBe("audio");
});
