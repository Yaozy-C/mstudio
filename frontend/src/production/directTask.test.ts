import { taskKey } from "./tasks";
import { expect, test } from "bun:test";
import { directTask, acceptDirectTask } from "./directTask";
import { fixture, model } from "./fixtures.test-helper";
import { saveTask } from "./document";
import { runsOf } from "./requestTask";
import { inputFor } from "./request";
import type { ProductionTask } from "./types";
test("direct execution freezes only explicit prompt, parameters and media, excluding script context", () => {
  const p = fixture();
  const m = model("minimax/h3/image-to-video");
  const draft: ProductionTask = {
    key: "draft",
    kind: "video",
    mode: "ends",
    modelId: m.id,
    prompt: "镜头推进",
    parameters: { duration: 8, resolution: "768P" },
    inputs: [
      {
        key: "script",
        role: "script",
        assetId: "",
        purpose: "历史脚本与上下文不能进入请求",
      },
      { key: "b", role: "last-frame", assetId: "b", purpose: "尾帧" },
      { key: "a", role: "first-frame", assetId: "a", purpose: "首帧" },
    ],
    instruction: "上一轮的要求",
    error: "旧错误",
    jobId: "旧任务",
  };
  const run = directTask(p, draft, m, "one");
  const request = inputFor(p, run, m, [
    { assetId: "a", kind: "image", url: "https://example.com/a" },
    { assetId: "b", kind: "image", url: "https://example.com/b" },
  ]);
  expect(run.status).toBe("READY");
  expect(run.instruction).toBe("镜头推进");
  expect(run.inputs).toHaveLength(3);
  expect(run.jobId).toBeUndefined();
  expect(request.prompt).not.toContain("历史");
  expect(request.prompt).not.toContain("上一轮");
  expect(request.duration).toBe(8);
  expect(request.image_url).toBe("https://example.com/a");
  expect(request.end_image_url).toBe("https://example.com/b");
  draft.inputs[0].assetId = "changed";
  expect(run.inputs[0].assetId).toBe("");
  expect(runsOf(saveTask(p, run))).toEqual([run]);
});
test("image execution uses selected media model without an Agent model", () => {
  const m = model("https://generativelanguage.googleapis.com/v1beta", "image");
  m.plugin = "gemini-native";
  m.params = { model: "gemini-3.1-flash-image-preview" };
  const task: ProductionTask = {
    key: "draft",
    kind: "image",
    mode: "multi",
    inputs: [],
    modelId: "",
    prompt: "白底产品图",
    parameters: { aspectRatio: "1:1" },
  };
  expect(directTask(fixture(), task, m, "image").modelId).toBe(m.id);
  expect(() =>
    directTask(fixture(), { ...task, prompt: "" }, m, "invalid"),
  ).toThrow();
  expect(() =>
    directTask(fixture(), task, { ...m, enabled: false }, "disabled"),
  ).toThrow();
});

test("accepting a direct task clears composer media, frames and owner but preserves the run", () => {
  const p = fixture(),
    m = model("minimax/h3/image-to-video");
  const draft: ProductionTask = {
    key: "selected-shot",
    ownerId: "shot",
    kind: "video",
    mode: "ends",
    modelId: m.id,
    prompt: "缓慢推进",
    parameters: { duration: 8 },
    inputs: [
      { key: "a", assetId: "a", role: "first-frame", purpose: "首帧" },
      { key: "b", assetId: "b", role: "last-frame", purpose: "尾帧" },
    ],
  };
  const run = directTask(p, draft, m, "clear");
  const next = acceptDirectTask(p, draft, run);
  for (const key of [draft.key, taskKey([])]) {
    const cleared = next.production!.drafts![key];
    expect(cleared.inputs).toEqual([]);
    expect(cleared.prompt).toBe("");
    expect(cleared.ownerId).toBeUndefined();
    expect(cleared.modelId).toBe(m.id);
    expect(cleared.parameters).toEqual({ duration: 8 });
  }
  expect(next.production!.drafts![run.key].inputs).toHaveLength(2);
  expect(next.production!.drafts![run.key].prompt).toBe("缓慢推进");
  expect(draft.inputs).toHaveLength(2);
});
