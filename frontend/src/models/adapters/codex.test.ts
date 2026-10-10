import { expect, test } from "bun:test";
import { mediaAdapter } from ".";
import { validateMediaModel } from "../mediaRegistry";
import { modelLibrary } from "../catalogSpecs";
import type { MediaModel } from "../mediaRegistry";
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
      options: { model: "codex-image", n: 1 },
      inputs: [
        { kind: "image", role: "reference", url: "data:image/png;base64,YQ==" },
      ],
    }),
  ).toEqual({
    prompt: "test",
    model: "codex-image",
    n: 1,
    image: ["data:image/png;base64,YQ=="],
  });
  expect(() =>
    mediaAdapter(model).encode({
      prompt: "test",
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
