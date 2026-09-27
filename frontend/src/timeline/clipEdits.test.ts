import { expect, test } from "bun:test";
import { newProject, makeClip, type Asset } from "../model";
import { editKey, pasteClip, trimAtPlayhead } from "./clipEdits";
const asset: Asset = {
  id: "source",
  name: "v",
  kind: "video",
  path: "",
  preview: "",
  duration: 12,
  width: 320,
  height: 240,
  hasAudio: true,
};
function fixture() {
  return {
    ...newProject("edits"),
    assets: [asset],
    clips: [
      {
        ...makeClip(asset),
        id: "c",
        start: 2,
        trimIn: 2,
        trimOut: 10,
        speed: 2,
        volume: 0.6,
        visual: {
          brightness: 0,
          contrast: 1.1,
          saturation: 0.8,
          temperature: 0.2,
          effect: "none" as const,
        },
      },
    ],
  };
}
test("trim at playhead changes source boundaries at existing speed and leaves other edits alone", () => {
  const p = fixture();
  const left = trimAtPlayhead(p, "c", 3, "trim-start");
  expect(left.clips[0]).toEqual({
    ...p.clips[0],
    start: 3,
    trimIn: 4,
    transition: undefined,
  });
  const right = trimAtPlayhead(p, "c", 3, "trim-end");
  expect(right.clips[0]).toEqual({ ...p.clips[0], trimOut: 4 });
  expect(right.tracks).toBe(p.tracks);
  expect(right.captions).toBe(p.captions);
  for (const t of [NaN, 0, 2, 6, 10])
    expect(trimAtPlayhead(p, "c", t, "trim-start")).toBe(p);
  expect(p.clips[0].trimOut).toBe(10);
});
test("paste uses distinct IDs and a frame-aligned playhead without changing speed, sound or grading", () => {
  const p = fixture();
  const saved = {
    ...p.clips[0],
    transition: { fromClipId: "left", kind: "fade" as const, duration: 0.5 },
  };
  const a = pasteClip(p, saved, 7.014),
    b = pasteClip(a.project, saved, 9);
  expect(a.id).not.toBe(saved.id);
  expect(b.id).not.toBe(a.id);
  expect(a.project.clips[1]).toEqual({
    ...saved,
    id: a.id!,
    start: 7,
    transition: undefined,
  });
  expect(a.project.clips[1].visual).not.toBe(saved.visual);
  expect(p.clips).toHaveLength(1);
  expect(pasteClip({ ...p, assets: [] }, saved, 0).project.clips).toHaveLength(
    1,
  );
  expect(pasteClip({ ...p, tracks: [] }, saved, 0).id).toBeUndefined();
});
test("editing key mapping distinguishes cut, split modifiers and platform keys", () => {
  const key = (key: string, extra = {}) =>
    editKey({
      key,
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      ...extra,
    });
  expect(key("c", { metaKey: true })).toBe("copy");
  expect(key("X", { ctrlKey: true })).toBe("cut");
  expect(key("v", { metaKey: true })).toBe("paste");
  expect(key("[", { altKey: true })).toBe("trim-start");
  expect(key("]", { altKey: true })).toBe("trim-end");
  expect(key("c")).toBeUndefined();
  expect(key("c", { metaKey: true, shiftKey: true })).toBeUndefined();
  expect(key("b", { metaKey: true })).toBeUndefined();
});
