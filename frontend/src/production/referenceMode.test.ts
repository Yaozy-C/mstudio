import { expect, test } from "bun:test";
import type { bridge } from "../bridge";
import { fixture, model } from "./fixtures.test-helper";
import { preparePrompt } from "./preparePrompt";
import { directTask } from "./directTask";
import { inputFor } from "./request";
import { referenceIssue, supportsVideoReference } from "./referenceMode";
import type { ProductionTask } from "./types";

const draft: ProductionTask = {
  key: "draft",
  kind: "video",
  mode: "mixed",
  modelId: "model",
  prompt: "保留横移和揭晓，换成我的商品。",
  parameters: { duration: 6 },
  inputs: [
    {
      key: "v",
      assetId: "v",
      role: "video-reference",
      purpose: "运镜",
      start: 3,
      end: 9,
    },
    { key: "a", assetId: "a", role: "reference", purpose: "商品外观" },
  ],
};

test("reference submission skips the text model and preserves the exact prompt, range and media roles", async () => {
  let calls = 0;
  const invoke = (async () => {
    calls++;
    throw new Error("No chat model configured");
  }) as typeof bridge;
  const prepared = await preparePrompt(invoke, "project", draft, true);
  expect(calls).toBe(0);
  expect(prepared).toEqual(draft);
  const m = model("minimax/h3/reference-to-video");
  const run = directTask(fixture(), prepared, m, "reference");
  expect(run.prompt).toBe(draft.prompt);
  expect(run.inputs).toEqual(draft.inputs);
  const request = inputFor(fixture(), run, m, [
    { assetId: "v", kind: "video", url: "https://example.com/clip.mp4" },
    { assetId: "a", kind: "image", url: "https://example.com/product.png" },
  ]) as Record<string, unknown>;
  expect(request.reference_video_urls).toEqual([
    "https://example.com/clip.mp4",
  ]);
  expect(request.prompt).toContain(draft.prompt);
  prepared.inputs[0].start = 0;
  expect(draft.inputs[0].start).toBe(3);
  expect(run.inputs[0].start).toBe(3);
});

test("reference mode requires video input and rejects text context or incompatible models", async () => {
  expect(supportsVideoReference(model("minimax/h3/reference-to-video"))).toBe(
    true,
  );
  expect(supportsVideoReference(model("minimax/h3/image-to-video"))).toBe(
    false,
  );
  expect(referenceIssue(draft, model("minimax/h3/image-to-video"))).not.toBe(
    "",
  );
  const invoke = (async () => {
    throw new Error("must not call");
  }) as typeof bridge;
  for (const inputs of [
    draft.inputs.slice(1),
    [
      ...draft.inputs,
      {
        key: "s",
        assetId: "",
        role: "script" as const,
        purpose: "brief",
        nodeId: "screenplay",
      },
    ],
  ]) {
    await expect(
      preparePrompt(invoke, "p", { ...draft, inputs }, true),
    ).rejects.toThrow();
  }
  expect(() =>
    directTask(
      fixture(),
      { ...draft, inputs: [{ ...draft.inputs[0], end: 30 }] },
      model("minimax/h3/reference-to-video"),
      "bad-range",
    ),
  ).toThrow();
});
