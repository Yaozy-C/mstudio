import { expect, test } from "bun:test";
import { fixture, model } from "./fixtures.test-helper";
import {
  generationPrompt,
  inputFor,
  mediaReferences,
  submittedPrompt,
} from "./request";
import type { ProductionTask } from "./types";

test("submitted image and video prompts exclude script notes and future actions", () => {
  const p = fixture();
  p.nodes[1].text = "【摄影运镜】移焦；随后手伸入并冒出冰雾";
  p.nodes[0].screenplay!.script![0].dialogue = "38℃的盛夏";
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
  // A role label is not a purpose: a legacy label, or no purpose at all, is
  // rendered once instead of being duplicated into "内容参考 · 内容参考".
  for (const purpose of ["内容参考", "", "内容参考 · 产品身份"]) {
    const legacy = {
      ...task,
      inputs: [{ key: "b", assetId: "b", role: "reference" as const, purpose }],
    };
    const media = mediaReferences(legacy);
    expect(media[0].purpose).not.toContain("内容参考 · 内容参考");
    expect(media[0].purpose.startsWith("内容参考")).toBe(true);
  }
  expect(mediaReferences(task)[1].purpose).toBe("内容参考 · 产品身份");
  p.nodes[1].text = "Entirely changed direction";
  p.nodes[0].screenplay!.script![0].action = "Unrelated new action";
  expect(inputFor(p, task, m)).toEqual(first);
});

test("the audited task's generic reference labels no longer duplicate the role", () => {
  // Real submission for job 1791390322635289000-7 (project 10-7-2): seven inputs,
  // where only the product image carried a concrete purpose and the other six were
  // stored as the generic label. That rendered as "内容参考 · 内容参考" six times and
  // left the recurring people and views unassigned.
  const product =
    "Original product geometry and materials: blue textured rectangular lunch bag, black top zipper with two silver pulls, black handles with blue padded grip, front zip pocket, side mesh pockets and clipped shoulder strap. Preserve proportions and components. Exclude white background and FRONT label.";
  const task: ProductionTask = {
    key: "audited",
    kind: "video",
    mode: "multi",
    modelId: "test",
    ownerId: "shot",
    prompt:
      "Generate a 15-second 9:16 vertical photorealistic live-action TikTok advertisement for the US market.",
    inputs: [
      { key: "0", assetId: "a0", role: "reference", purpose: product },
      ...Array.from({ length: 6 }, (_, n) => ({
        key: `${n + 1}`,
        assetId: `a${n + 1}`,
        role: "reference" as const,
        purpose: "内容参考",
      })),
    ],
  };
  const media = mediaReferences(task);
  expect(media).toHaveLength(7);
  // The concrete product purpose survives verbatim.
  expect(media[0].purpose).toBe(`内容参考 · ${product}`);
  // The generic label collapses to the role label instead of doubling it.
  expect(media.slice(1).map((r) => r.purpose)).toEqual(
    Array(6).fill("内容参考"),
  );
  expect(media.some((r) => r.purpose.includes("内容参考 · 内容参考"))).toBe(
    false,
  );
});
