import { expect, test } from "bun:test";
import { model, fixture } from "./fixtures.test-helper";
import { parameterFields, taskParameters } from "./parameters";
import { inputFor } from "./request";

test("Gemini overrides preserve the model's other generation settings", () => {
  const m = model("gemini", "image");
  m.plugin = "gemini-native";
  m.params = {
    generationConfig: { temperature: 0.7, imageConfig: { imageSize: "1K" } },
  };
  expect(taskParameters(m, { aspectRatio: "9:16", resolution: "2K" })).toEqual({
    generationConfig: {
      temperature: 0.7,
      imageConfig: { imageSize: "2K", aspectRatio: "9:16" },
    },
  });
});
test("video settings reach the request with the exact first and last frames", () => {
  const m = model("minimax/h3/image-to-video");
  const result = inputFor(
    fixture(),
    {
      key: "t",
      ownerId: "shot",
      kind: "video",
      mode: "ends",
      modelId: m.id,
      prompt: "缓慢推进",
      parameters: { duration: 8, resolution: "768P" },
      inputs: [
        { key: "a", assetId: "a", role: "first-frame", purpose: "首帧" },
        { key: "b", assetId: "b", role: "last-frame", purpose: "尾帧" },
      ],
    },
    m,
    [
      { assetId: "a", kind: "image", url: "https://example.com/a.png" },
      { assetId: "b", kind: "image", url: "https://example.com/b.png" },
    ],
  );
  expect(result.duration).toBe(8);
  expect(result.resolution).toBe("768P");
  expect(result.image_url).toBeTruthy();
  expect(result.end_image_url).toBeTruthy();
  expect(result.image_url).not.toBe(result.end_image_url);
  expect(parameterFields(m).ratios).toEqual([]);
});
test("invalid and unsupported settings are rejected before submission", () => {
  const m = model("openai/gpt-image-2.5/flare/edit", "image");
  expect(taskParameters(m, { width: 1024, height: 1536 })).toEqual({
    image_size: { width: 1024, height: 1536 },
  });
  expect(() => taskParameters(m, { width: 1000, height: 1024 })).toThrow();
  expect(() => taskParameters(model("custom"), { duration: 8 })).toThrow();
  expect(() =>
    taskParameters(model("minimax/h3/image-to-video"), { duration: 16 }),
  ).toThrow();
});
