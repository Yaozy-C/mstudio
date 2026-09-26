import { expect, test } from "bun:test";
import { modelAdapter } from ".";
import { validateMediaModel } from "../mediaRegistry";
import { modelLibrary } from "../catalogSpecs";
test("Codex is selectable without an API service and uses Images payload fields", () => {
  const spec = modelLibrary.find((s) => s.id === "codex-image")!;
  expect(spec.endpoint).toBe("codex://local/images");
  validateMediaModel({
    id: "codex",
    name: spec.name,
    kind: "image",
    plugin: "codex-image",
    endpoint: spec.endpoint!,
    params: spec.request,
    enabled: true,
  });
  const adapter = modelAdapter("codex-image", spec.endpoint!);
  expect(
    adapter.encode({
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
    adapter.encode({
      prompt: "test",
      inputs: [
        { kind: "image", role: "reference", url: "https://example.com/a.png" },
      ],
    }),
  ).toThrow("项目图片");
});
