import { expect, test } from "bun:test";
import { mediaAdapter } from ".";
import { validateMediaModel } from "../mediaRegistry";
import { modelLibrary } from "../catalogSpecs";
import type { MediaModel } from "../mediaRegistry";
import { codexImageModel } from "../codexImageModel";
import { presetRequest } from ".";
const codexModel = (): MediaModel => {
  const spec = modelLibrary.find((s) => s.id === "codex-image")!;
  return {
    id: "codex",
    name: spec.name,
    kind: "image",
    plugin: "codex-image",
    endpoint: spec.endpoint!,
    params: spec.request,
    enabled: true,
    capabilities: spec.capabilities,
  };
};
test("Codex passes more than five reference images without truncation", () => {
  const inputs = Array.from({ length: 12 }, () => ({
    kind: "image" as const,
    role: "reference" as const,
    url: "data:image/png;base64,YQ==",
  }));
  const result = mediaAdapter(codexModel()).encode({
    prompt: "Use every reference",
    options: codexModel().params,
    inputs,
  });
  expect(result.image).toHaveLength(12);
});
test("Codex is selectable without an API service and uses Images payload fields", () => {
  const model = codexModel();
  expect(model.endpoint).toBe("codex://local/images");
  validateMediaModel(model);
  expect(
    mediaAdapter(model).encode({
      prompt: "test",
      options: { model: "gpt-image-2.5-sunburst", n: 1 },
      inputs: [
        { kind: "image", role: "reference", url: "data:image/png;base64,YQ==" },
      ],
    }),
  ).toEqual({
    prompt: "test",
    model: "gpt-image-2.5-sunburst",
    n: 1,
    image: ["data:image/png;base64,YQ=="],
  });
  expect(() =>
    mediaAdapter(model).encode({
      prompt: "test",
      options: model.params,
      inputs: [
        { kind: "image", role: "reference", url: "https://example.com/a.png" },
      ],
    }),
  ).toThrow("项目图片");
});
test("Codex rejects a reference kind its request builder cannot embed", () => {
  const model = codexModel();
  expect(() =>
    validateMediaModel({
      ...model,
      capabilities: {
        references: [{ key: "/clip", kind: "video", role: "reference" }],
      },
    }),
  ).toThrow("图片参考");
});

test("Codex catalog requests Sunburst and preserves selected IDs in both input modes", () => {
  expect(codexModel().params.model).toBe("gpt-image-2.5-sunburst");
  for (const id of [
    "gpt-image-2.5-sunburst",
    "gpt-image-2.5-flare",
    "gpt-image-2",
  ]) {
    for (const edit of [false, true]) {
      const model = { ...codexModel(), params: { model: id } };
      validateMediaModel(model);
      const adapter = mediaAdapter(model);
      const params = {
        ...model.params,
        ...(edit ? { image: ["data:image/png;base64,YQ=="] } : {}),
      };
      const encoded = adapter.encode(presetRequest(adapter, "test", params));
      expect(encoded.model).toBe(id);
      expect(encoded.image).toEqual(edit ? params.image : undefined);
    }
  }
});

test("Codex requires an explicit model on save and request construction", () => {
  expect(() => codexImageModel({})).toThrow("模型 ID");
  for (const id of [
    undefined,
    "codex-image",
    null,
    42,
    "",
    "gpt-image-",
    "gpt-image-2\n",
    "gpt-image-2/other",
    "gpt-image-" + "x".repeat(100),
  ]) {
    expect(() =>
      validateMediaModel({ ...codexModel(), params: { model: id } }),
    ).toThrow("模型 ID");
    expect(() =>
      mediaAdapter(codexModel()).encode({
        prompt: "test",
        options: { model: id },
      }),
    ).toThrow("模型 ID");
  }
});
