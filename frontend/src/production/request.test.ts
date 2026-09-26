import { expect, test } from "bun:test";
import { fixture, model } from "./fixtures.test-helper";
import { generationPrompt, inputFor, submittedPrompt } from "./request";
import type { ProductionTask } from "./types";

test("submitted image and video prompts exclude script notes and future actions", () => {
  const p = fixture();
  p.nodes[1].text = "【摄影运镜】移焦；随后手伸入并冒出冰雾";
  p.nodes[0].plan!.script![0].dialogue = "38℃的盛夏";
  for (const kind of ["image", "video"] as const) {
    const task: ProductionTask = {
      key: "clean",
      ownerId: p.nodes[1].id,
      kind,
      mode: "single",
      modelId: "test",
      inputs: [
        {
          key: "script",
          assetId: "",
          role: "script",
          purpose: "【入出镜状态】开包喷雾",
        },
      ],
      prompt:
        kind === "image"
          ? "一张满幅实拍画面，闭合的蓝包放在草地上。"
          : "一个连续镜头，手提起包。",
    };
    expect(generationPrompt(p, task)).toBe(task.prompt);
    const m = model("minimax/h3/text-to-video");
    if (kind === "image") {
      m.kind = "image";
      m.endpoint = "fal-ai/flux/schnell";
    }
    expect(inputFor(p, task, m).prompt).toBe(task.prompt);
  }
});

test("displayed submission preserves explicit typography and reference order without live script injection", () => {
  const p = fixture();
  const task: ProductionTask = {
    key: "poster",
    kind: "image",
    mode: "multi",
    modelId: "test",
    ownerId: "shot",
    prompt: "制作海报，标题为“夏日”。",
    inputs: [
      { key: "b", assetId: "b", role: "reference", purpose: "构图" },
      { key: "a", assetId: "a", role: "reference", purpose: "产品身份" },
    ],
  };
  const m = model("fal-ai/flux-2/edit", "image");
  m.plugin = "gemini-native";
  m.endpoint = "https://generativelanguage.googleapis.com";
  m.params = { model: "gemini-3.1-flash-image" };
  const first = inputFor(p, task, m);
  expect(first.prompt).toBe(submittedPrompt(p, task));
  expect(first.prompt).toContain("标题为“夏日”");
  expect(first.prompt).toContain("Image 1: 内容参考 · 构图");
  expect(first.prompt).toContain("Image 2: 内容参考 · 产品身份");
  p.nodes[1].text = "Entirely changed direction";
  p.nodes[0].plan!.script![0].action = "Unrelated new action";
  expect(inputFor(p, task, m)).toEqual(first);
});
