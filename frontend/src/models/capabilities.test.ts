import { expect, test } from "bun:test";
import {
  parseCapabilities,
  pointerDelete,
  pointerGet,
  pointerSet,
  referenceLimits,
  resolvedControls,
  validateCapabilities,
} from "./capabilities";
import { modelLibrary } from "./catalogSpecs";
import { catalogMediaModel } from "./catalogMediaModel";
import { validateMediaModel } from "./validateMediaModel";

test("pointers read, write and delete through nested request bodies", () => {
  const body: Record<string, unknown> = {
    input: { prompt: "p", img_url: "https://example.com/a.png" },
    parameters: { duration: 5 },
  };
  expect(pointerGet(body, "/input/img_url")).toBe("https://example.com/a.png");
  expect(pointerGet(body, "/parameters/duration")).toBe(5);
  expect(pointerGet(body, "/input/missing")).toBeUndefined();
  expect(pointerGet(body, "/input/prompt/deeper")).toBeUndefined();
  pointerSet(body, "/parameters/ratio", "16:9");
  expect(body.parameters).toEqual({ duration: 5, ratio: "16:9" });
  pointerSet(body, "/input/nested/deep", 1);
  expect(pointerGet(body, "/input/nested/deep")).toBe(1);
  pointerDelete(body, "/input/img_url");
  expect(pointerGet(body, "/input/img_url")).toBeUndefined();
  expect(pointerGet(body, "/input/prompt")).toBe("p");
  // Escaped separators stay part of the key, per RFC 6901.
  pointerSet(body, "/a~1b/c~0d", true);
  expect(pointerGet(body, "/a~1b/c~0d")).toBe(true);
  expect(() => pointerGet(body, "input/prompt")).toThrow("JSON Pointer");
});

test("a declaration is data: unknown fields, roles, kinds and limits are rejected", () => {
  expect(
    validateCapabilities({
      references: [{ key: "/image_url", kind: "image", role: "first-frame" }],
      controls: { duration: { path: "/duration", min: 2, max: 15 } },
      referenceLimit: 2,
      referenceSeconds: 10,
    }),
  ).toEqual({
    references: [{ key: "/image_url", kind: "image", role: "first-frame" }],
    controls: { duration: { path: "/duration", min: 2, max: 15 } },
    referenceLimit: 2,
    referenceSeconds: 10,
  });
  for (const rejected of [
    { references: [{ key: "image_url", kind: "image", role: "reference" }] },
    { references: [{ key: "/a", kind: "image", role: "nope" }] },
    { references: [{ key: "/a", kind: "clip", role: "reference" }] },
    { references: [{ key: "/a", kind: "image", role: "reference", max: 0 }] },
    {
      references: [
        { key: "/a", kind: "image", role: "reference" },
        { key: "/a", kind: "image", role: "reference" },
      ],
    },
    { references: [{ key: "/a", kind: "image", role: "reference", extra: 1 }] },
    { controls: { ratio: { path: "/ratio", values: ["1:1"] } } },
    { controls: { aspectRatio: { path: "aspect_ratio", values: ["1:1"] } } },
    { controls: { aspectRatio: { path: "/aspect_ratio" } } },
    { controls: { duration: { path: "/duration", values: ["5"] } } },
    { controls: { imageSize: { path: "/image_size", values: ["1K"] } } },
    { controls: { duration: { path: "/duration", min: 9, max: 5 } } },
    { referenceLimit: 13 },
    { referenceSeconds: 16 },
    { unexpected: true },
  ])
    expect(() => validateCapabilities(rejected)).toThrow();
  expect(parseCapabilities("")).toBeUndefined();
  expect(parseCapabilities("{}")).toBeUndefined();
  expect(() => parseCapabilities("{")).toThrow("有效的 JSON");
});

test("every shipped library declaration is valid and reaches the added model", () => {
  for (const spec of modelLibrary) {
    if (!spec.endpoint) continue;
    const plain = catalogMediaModel(
      spec,
      spec.kind === "video" ? "video" : "image",
    );
    expect(plain.capabilities).toEqual(spec.capabilities);
    const editing = catalogMediaModel(
      spec,
      spec.kind === "video" ? "video" : "image",
      true,
    );
    expect(editing.capabilities).toEqual(
      spec.editCapabilities ?? spec.capabilities,
    );
    for (const candidate of [plain, editing]) {
      // Full save-path validation, including the protocol guards.
      validateMediaModel({ ...candidate, name: candidate.name || "model" });
    }
  }
});

test("first-frame models report a frame-derived ratio instead of offering ratios", () => {
  const wan = {
    plugin: "fal",
    capabilities: {
      references: [
        {
          key: "/image_url",
          kind: "image" as const,
          role: "first-frame" as const,
        },
      ],
      controls: {
        duration: { path: "/duration", min: 2, max: 15 },
      },
    },
  };
  const controls = resolvedControls(wan);
  expect(controls.aspectRatio).toBeNull();
  expect(referenceLimits(wan)).toEqual({
    count: null,
    seconds: null,
    kinds: { image: null, video: null, audio: null },
  });
  const declared = {
    plugin: "fal",
    capabilities: {
      references: [
        {
          key: "/image_url",
          kind: "image" as const,
          role: "reference" as const,
          max: 2,
        },
        {
          key: "/clip",
          kind: "video" as const,
          role: "reference" as const,
          max: 3,
        },
      ],
      referenceLimit: 4,
      referenceSeconds: 8,
    },
  };
  expect(referenceLimits(declared)).toEqual({
    count: 4,
    seconds: 8,
    kinds: { image: 2, video: 3, audio: null },
  });
});
