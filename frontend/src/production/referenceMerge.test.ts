import { expect, test } from "bun:test";
import { asset, fixture, model } from "./fixtures.test-helper";
import { inputFor } from "./request";
import { createTask } from "./tasks";
import type { ProductionTask } from "./types";

function setup(durations = [4 / 3, 4 / 3]) {
  const project = fixture();
  const videos = durations.map((duration, i) => ({
    ...asset(`video-${i}`, "video"),
    duration,
  }));
  project.assets.push(...videos);
  const task: ProductionTask = {
    ...createTask(project, [], "video"),
    prompt: "保留动作顺序",
    inputs: [
      { key: "a", assetId: "a", role: "reference" as const, purpose: "产品" },
      ...videos.map((video, i) => ({
        key: video.id,
        assetId: video.id,
        role: "video-reference" as const,
        purpose: `动作 ${i + 1}`,
        start: 0,
        end: video.duration,
      })),
    ],
  };
  return { project, task };
}

test("short video references merge into one URL before submission with segment purposes", () => {
  const { project, task } = setup();
  const m = model("minimax/h3/reference-to-video");
  m.capabilities = structuredClone(m.capabilities);
  m.capabilities!.referenceLimit = 2;
  const preflight = inputFor(project, task, m);
  expect(preflight.reference_video_urls).toHaveLength(1);
  const result = inputFor(project, task, m, [
    { assetId: "a", kind: "image", url: "https://test/product.png" },
    {
      assetId: "video-0",
      sourceAssetIds: ["video-0", "video-1"],
      kind: "video",
      url: "https://test/merged.mp4",
    },
  ]);
  expect(result.reference_video_urls).toEqual(["https://test/merged.mp4"]);
  expect(result.reference_image_urls).toEqual(["https://test/product.png"]);
  expect(result.prompt).toContain("Video 1 (0.000–1.333s): 动作参考 · 动作 1");
  expect(result.prompt).toContain("Video 1 (1.333–2.667s): 动作参考 · 动作 2");
  expect(result.prompt).not.toContain("Video 2");
  expect(task.inputs[2].assetId).toBe("video-1");
});

test("clips at least two seconds remain separate references", () => {
  const { project, task } = setup([2, 3]);
  const result = inputFor(
    project,
    task,
    model("minimax/h3/reference-to-video"),
    [
      { assetId: "a", kind: "image", url: "https://test/image" },
      { assetId: "video-0", kind: "video", url: "https://test/first" },
      { assetId: "video-1", kind: "video", url: "https://test/second" },
    ],
  );
  expect(result.reference_video_urls).toEqual([
    "https://test/first",
    "https://test/second",
  ]);
  expect(result.prompt).toContain("Video 2: 动作参考 · 动作 2");
});

test("a short trim triggers merging and separate uploads cannot bypass the merge", () => {
  const { project, task } = setup([6, 4]);
  task.inputs[1].start = 3;
  task.inputs[1].end = 4;
  const m = model("minimax/h3/reference-to-video");
  expect(inputFor(project, task, m).reference_video_urls).toHaveLength(1);
  expect(() =>
    inputFor(project, task, m, [
      { assetId: "a", kind: "image", url: "https://test/image" },
      { assetId: "video-0", kind: "video", url: "https://test/first" },
      { assetId: "video-1", kind: "video", url: "https://test/second" },
    ]),
  ).toThrow("合并参考视频尚未上传");
});
