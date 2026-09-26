import { expect, test } from "bun:test";
import { newProject, type Asset } from "../model";
import { parseSrt, toSrt } from "./captions";
import { registeredInput } from "./generationInput";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
const asset: Asset = {
  id: "video",
  name: "video",
  kind: "video",
  duration: 20,
  hasAudio: true,
  path: "",
  preview: "",
  width: 640,
  height: 360,
};
test("SRT roundtrips milliseconds and rejects malformed timestamps atomically", () => {
  const captions = parseSrt(
    "1\r\n00:00:01,025 --> 00:00:02,300\r\n第一句\r\nSecond line\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,020\r\n第二句",
  );
  expect(captions[0].start).toBe(1.025);
  expect(captions[0].text).toContain("Second line");
  expect(parseSrt(toSrt(captions)).map(({ id, ...c }) => c)).toEqual(
    captions.map(({ id, ...c }) => c),
  );
  expect(() => parseSrt("00:61:00,000 --> 00:62:00,000\na")).toThrow();
  expect(() => parseSrt("00:00:02,000 --> 00:00:01,000\na")).toThrow();
});
test("reference preparation validates before uploads and keeps modality order and explicit reference roles", () => {
  const image: Asset = { ...asset, id: "image", kind: "image" };
  const p = {
    ...newProject("video"),
    assets: [asset, image],
    creation: {
      intent: "轻快",
      essential: "饮料跳进包",
      preserve: "保留网袋",
      stage: "production" as const,
    },
  };
  const refs = [
    { assetId: "image", purpose: "包的外观" },
    { assetId: "video", purpose: "动作节奏", start: 2, end: 7 },
  ];
  const input = registeredInput(model, p, "镜头动作", refs, [
    {
      assetId: "image",
      kind: "image",
      role: "reference",
      url: "https://test/image",
    },
    {
      assetId: "video",
      kind: "video",
      role: "reference",
      url: "https://test/video",
    },
  ]);
  expect(input.prompt).not.toContain("饮料跳进包");
  expect(input.prompt).toContain("Video 1: 动作节奏");
  expect(input.reference_video_urls).toEqual(["https://test/video"]);
});
test("Agent modifies scoped multitrack edits and captions atomically with range validation", () => {
  const p = { ...newProject("test"), assets: [asset] };
  const updated = applyOperations(p, 0, [
    { op: "set_creation", essential: "不能删关键动作" },
    { op: "add_track", id: "overlay", trackKind: "video", title: "叠加" },
    {
      op: "append_clip",
      assetId: "video",
      trackId: "overlay",
      start: 3,
      trimOut: 5,
      scale: 0.5,
    },
    { op: "add_caption", id: "sub", text: "字幕", start: 3, end: 5 },
  ]);
  expect(updated.clips[0].start).toBe(3);
  expect(updated.captions![0].text).toBe("字幕");
  expect(inspectProject(updated, {}).creation?.essential).toBe(
    "不能删关键动作",
  );
  expect(() =>
    applyOperations(updated, 0, [
      { op: "set_creation", intent: "other" },
      { op: "update_clip", id: updated.clips[0].id, speed: 8 },
    ]),
  ).toThrow();
  expect(updated.creation?.intent).toBe("");
});
test("Agent context can retrieve long decisions in bounded pages", () => {
  const p = {
    ...newProject("context"),
    creation: {
      intent: "目标".repeat(3000),
      essential: "关键事件".repeat(1000),
      preserve: "保持不变".repeat(1000),
      stage: "editing" as const,
    },
  };
  const summary = inspectProject(p, {});
  expect(JSON.stringify(summary).length).toBeLessThan(6000);
  expect(JSON.stringify(summary)).not.toContain("只修改当前镜头");
  const page = inspectProject(p, { section: "creation", textOffset: 800 });
  expect(page.creation?.intent).toEqual({
    text: p.creation.intent.slice(800, 1600),
    nextTextOffset: 1600,
  });
});

const model = {
  id: "h3",
  name: "H3",
  plugin: "fal",
  kind: "video" as const,
  endpoint: "minimax/h3/reference-to-video",
  params: { duration: 6 },
  enabled: true,
};

test("model adapter rejects unsupported local references and invalid ranges before upload", () => {
  const p = { ...newProject("test"), assets: [asset] };
  const refs = [{ assetId: "video", purpose: "motion", start: 0, end: 25 }];
  const files = [
    {
      assetId: "video",
      kind: "video" as const,
      role: "reference" as const,
      url: "https://example.test/ref",
    },
  ];
  expect(() => registeredInput(model, p, "generate", refs, files)).toThrow(
    "区间",
  );
  expect(() =>
    registeredInput(
      { ...model, endpoint: "fal-ai/nano-banana-2" },
      p,
      "generate",
      [{ ...refs[0], end: 5 }],
      files,
    ),
  ).toThrow("不支持");
  expect(() =>
    registeredInput(model, p, "generate", [{ ...refs[0], end: 5 }], []),
  ).toThrow("尚未上传");
});
