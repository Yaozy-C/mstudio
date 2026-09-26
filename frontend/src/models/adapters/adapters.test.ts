import { expect, test } from "bun:test";
import { modelAdapter, presetRequest, type ModelRequest } from "./index";
const input: ModelRequest = {
  prompt: "保留主体，修改背景",
  inputs: [
    {
      kind: "image",
      role: "reference",
      url: "https://example.com/product.png",
    },
  ],
  options: { num_images: 1 },
};
test("business requests are shared by different image models", () => {
  const gpt = modelAdapter("fal", "openai/gpt-image-2.5/sunburst/edit");
  const banana = modelAdapter("fal", "fal-ai/nano-banana-2/edit");
  expect(gpt.encode(input)).toEqual(banana.encode(input));
  expect(gpt.encode(input).image_urls).toEqual([
    "https://example.com/product.png",
  ]);
  expect(input.inputs?.[0].role).toBe("reference");
});
test("adapters reject unsupported reference roles rather than silently dropping them", () => {
  expect(() =>
    modelAdapter("fal", "minimax/h3/text-to-video").encode(input),
  ).toThrow("不支持");
  expect(() =>
    modelAdapter("fal", "minimax/h3-max/image-to-video").encode(input),
  ).toThrow("不支持");
  expect(() => modelAdapter("uninstalled", "model")).toThrow("尚未安装");
});
test("model form presets are decoded at the boundary without mutating saved parameters", () => {
  const adapter = modelAdapter("fal", "openai/gpt-image-2.5/flare/edit");
  const params = {
    image_urls: ["https://example.com/image.png"],
    quality: "high",
    prompt: "old",
  };
  expect(adapter.encode(presetRequest(adapter, "new", params))).toEqual({
    ...params,
    prompt: "new",
  });
  expect(params.prompt).toBe("old");
});

test("custom HTTP templates preserve JSON types and escape prompt safely", () => {
  const adapter = modelAdapter("http-json", "https://example.com/generate");
  expect(
    adapter.encode({
      prompt: 'a "quote"\nline',
      options: {
        model: "new-model",
        input: { text: "{{prompt}}" },
        count: 2,
        flags: [true],
      },
    }),
  ).toEqual({
    model: "new-model",
    input: { text: 'a "quote"\nline' },
    count: 2,
    flags: [true],
  });
  expect(() =>
    adapter.encode({
      prompt: "x",
      inputs: [
        {
          kind: "image",
          role: "reference",
          url: "https://example.com/ref.png",
        },
      ],
    }),
  ).toThrow();
});

test("H3 maps first/last frames and soundtrack independently", () => {
  const adapter = modelAdapter("fal", "minimax/h3/image-to-video");
  const params = {
    image_url: "https://example.com/first.png",
    end_image_url: "https://example.com/last.png",
    target_audio_url: "https://example.com/voice.mp3",
    duration: 5,
  };
  expect(adapter.encode(presetRequest(adapter, "move", params))).toEqual({
    ...params,
    prompt: "move",
  });
});
test("H3 full references retain all modalities and enforce file limits", () => {
  const adapter = modelAdapter("fal", "minimax/h3/reference-to-video");
  const params = {
    reference_image_urls: ["https://example.com/i.png"],
    reference_video_urls: ["https://example.com/v.mp4"],
    reference_audio_urls: ["https://example.com/a.mp3"],
  };
  expect(
    adapter.encode(presetRequest(adapter, "Image 1 Video 1 Audio 1", params)),
  ).toEqual({ ...params, prompt: "Image 1 Video 1 Audio 1" });
  expect(() =>
    adapter.encode(
      presetRequest(adapter, "x", {
        ...params,
        reference_audio_urls: Array(4).fill("https://example.com/a.mp3"),
      }),
    ),
  ).toThrow();
  expect(() =>
    adapter.encode(
      presetRequest(adapter, "x", {
        reference_image_urls: Array(9).fill("https://example.com/i.png"),
        reference_video_urls: Array(3).fill("https://example.com/v.mp4"),
        reference_audio_urls: Array(1).fill("https://example.com/a.mp3"),
      }),
    ),
  ).toThrow();
});

test("Google direct image generation accepts text and local images without fal", () => {
  const adapter = modelAdapter(
    "gemini-native",
    "https://generativelanguage.googleapis.com",
  );
  expect(
    adapter.encode({
      prompt: "draw",
      options: { model: "gemini-3.1-flash-image" },
    }),
  ).toEqual({ prompt: "draw", model: "gemini-3.1-flash-image" });
  const url = "data:image/png;base64,YQ==";
  expect(
    adapter.encode({
      ...input,
      inputs: [{ kind: "image", role: "reference", url }],
    }).image_urls,
  ).toEqual([url]);
  expect(() => adapter.encode(input)).toThrow("项目中的参考图片");
});
