import { expect, test } from "bun:test";
import { newProject, makeClip, type Asset } from "../model";
import { applyOperations } from "./projectCommands";
import { defaultVisual } from "../timeline/visualSettings";
const asset: Asset = {
  id: "v",
  name: "source",
  kind: "video",
  path: "",
  preview: "",
  duration: 5,
  width: 160,
  height: 120,
  hasAudio: true,
};
function fixture() {
  return {
    ...newProject("grade"),
    assets: [asset],
    clips: [
      {
        ...makeClip(asset),
        id: "c",
        visual: { ...defaultVisual, effect: "vignette" as const },
      },
    ],
  };
}
test("Agent grade merges absolute values without changing edits, sound or existing effects", () => {
  const p = fixture();
  const next = applyOperations(p, 0, [
    {
      op: "update_clip",
      id: "c",
      visual: { temperature: -0.2, saturation: 0.85 },
    },
  ]);
  expect(next.clips[0]).toEqual({
    ...p.clips[0],
    visual: { ...p.clips[0].visual, temperature: -0.2, saturation: 0.85 },
  });
  expect(p.clips[0].visual.temperature).toBe(0);
  expect(
    applyOperations(next, 0, [{ op: "update_clip", id: "c", visual: null }])
      .clips[0].visual,
  ).toBeUndefined();
});
test("Agent grade rejects unknown filters, invalid values and stale targets atomically", () => {
  const p = fixture();
  for (const visual of [
    { brightness: 8 },
    { saturation: NaN },
    { temperature: "cold" },
    { filter: "movie=/tmp/input" },
    { effect: "unknown" },
  ]) {
    expect(() =>
      applyOperations(p, 0, [{ op: "update_clip", id: "c", visual }]),
    ).toThrow();
  }
  expect(() =>
    applyOperations(p, 1, [
      { op: "update_clip", id: "c", visual: { contrast: 1.1 } },
    ]),
  ).toThrow("工程已变化");
  const audio = { ...p, clips: [{ ...p.clips[0], trackId: "a1" }] };
  expect(() =>
    applyOperations(audio, 0, [
      { op: "update_clip", id: "c", visual: { contrast: 1.1 } },
    ]),
  ).toThrow();
});
