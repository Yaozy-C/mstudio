import { registeredInput } from "../creation/generationInput";
import { newProject } from "../model";
import { expect, test } from "bun:test";
import { validateMediaModel, type MediaModel } from "./mediaRegistry";
const model: MediaModel = {
  id: "image",
  name: "Image",
  kind: "image",
  plugin: "fal",
  endpoint: "fal-ai/flux/schnell",
  params: { num_images: 1, prompt: "old prompt" },
  enabled: true,
};
test("generation uses the selected model parameters and latest prompt without mutating the saved preset", () => {
  expect(
    registeredInput(model, newProject("test"), "new scene", [], []),
  ).toEqual({
    num_images: 1,
    prompt: "new scene",
  });
  expect(model.params.prompt).toBe("old prompt");
  expect(() =>
    registeredInput(
      { ...model, enabled: false },
      newProject("test"),
      "new scene",
      [],
      [],
    ),
  ).toThrow("停用");
});
test("model registration rejects URLs, unsupported adapters and non-object parameters", () => {
  expect(() =>
    validateMediaModel({ ...model, endpoint: "https://example.com/key" }),
  ).toThrow();
  expect(() =>
    validateMediaModel({
      ...model,
      params: [] as unknown as Record<string, unknown>,
    }),
  ).toThrow();
  expect(() =>
    validateMediaModel({ ...model, plugin: "unknown" as "fal" }),
  ).toThrow();
});

test("versioned model IDs are allowed without permitting path traversal", () => {
  expect(() =>
    validateMediaModel({
      ...model,
      endpoint: "openai/gpt-image-2.5/sunburst/edit",
    }),
  ).not.toThrow();
  for (const endpoint of [
    "fal-ai/../secret",
    "fal-ai/.hidden",
    "fal-ai//model",
    "fal-ai/model?key=x",
  ])
    expect(() => validateMediaModel({ ...model, endpoint })).toThrow();
});
