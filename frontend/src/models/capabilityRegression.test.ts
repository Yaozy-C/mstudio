import { expect, test } from "bun:test";
import {
  pointerGet,
  pointerSet,
  pointerDelete,
  validateCapabilities,
} from "./capabilities";
import { mediaAdapter, presetRequest } from "./adapters";
import { validateMediaModel } from "./validateMediaModel";
import { fixture, model } from "../production/fixtures.test-helper";
import { createTask } from "../production/tasks";
import { inputFor } from "../production/request";
import type { MediaModel } from "./mediaRegistry";

test("nested reference encoding and preset extraction leave saved model settings intact", () => {
  const m: MediaModel = {
    ...model("fal-ai/flux/schnell", "image"),
    capabilities: {
      references: [
        {
          key: "/input/images",
          kind: "image",
          role: "reference",
          multiple: true,
        },
      ],
    },
  };
  const params = {
    input: { images: ["https://example.com/old.png"], other: 1 },
  };
  const adapter = mediaAdapter(m);
  const before = structuredClone(params);
  const decoded = presetRequest(adapter, "p", params);
  expect(params).toEqual(before);
  expect(decoded.inputs).toHaveLength(1);
  adapter.encode({ prompt: "p", options: params, inputs: [] });
  expect(params).toEqual(before);
  const p = fixture();
  const task = createTask(p, [], "image");
  task.prompt = "p";
  task.inputs = [];
  expect(inputFor(p, task, { ...m, params })).toEqual({
    prompt: "p",
    input: { other: 1 },
  });
  expect(params).toEqual(before);
});

test("first and last frame per-field limits do not become a one-image total limit", () => {
  const p = fixture();
  const task = createTask(p, [], "video");
  task.prompt = "p";
  task.mode = "ends";
  task.inputs = [
    { key: "a", assetId: "a", purpose: "first", role: "first-frame" },
    { key: "b", assetId: "b", purpose: "last", role: "last-frame" },
  ];
  const m: MediaModel = {
    ...model("minimax/h3/text-to-video", "video"),
    capabilities: {
      references: [
        { key: "/image_url", kind: "image", role: "first-frame", max: 1 },
        { key: "/end_image_url", kind: "image", role: "last-frame", max: 1 },
      ],
    },
  };
  const body = inputFor(p, task, m);
  expect(body.image_url).toBeDefined();
  expect(body.end_image_url).toBeDefined();
});

test("native builders reject declarations they would ignore", () => {
  for (const plugin of ["gemini-native", "codex-image"]) {
    const m: MediaModel = {
      ...model("fal-ai/flux/schnell", "image"),
      plugin,
      endpoint:
        plugin === "gemini-native"
          ? "https://generativelanguage.googleapis.com"
          : "codex://local/images",
      params: { model: "test" },
      capabilities: {
        references: [
          { key: "/ignored", kind: "image", role: "reference", multiple: true },
        ],
      },
    };
    expect(() => validateMediaModel(m)).toThrow("固定字段");
  }
  expect(() =>
    validateMediaModel({
      ...model("fal-ai/flux/schnell", "image"),
      plugin: "http-json",
      endpoint: "https://example.com",
      http: { outputPointer: "/url" },
      capabilities: { references: [] },
    }),
  ).toThrow("不能声明参考字段");
});

test("pointer traversal handles arrays and rejects malformed escapes and prototype writes", () => {
  const body: Record<string, unknown> = { instances: [{ prompt: "p" }] };
  pointerSet(body, "/instances/0/image", "ref");
  expect(pointerGet(body, "/instances/0/image")).toBe("ref");
  pointerDelete(body, "/instances/0/image");
  expect(body).toEqual({ instances: [{ prompt: "p" }] });
  pointerSet(body, "/input/0/url", "ref");
  expect(body.input).toEqual([{ url: "ref" }]);
  for (const path of [
    "/bad~2",
    "/__proto__/polluted",
    "/constructor/prototype/polluted",
  ])
    expect(() => pointerSet(body, path, true)).toThrow();
  for (const value of [
    { referenceLimit: 1.5 },
    { controls: { duration: { path: "/duration", min: -1 } } },
    { controls: { duration: { path: "/duration", max: Infinity } } },
  ])
    expect(() => validateCapabilities(value)).toThrow();
});

test("disabling native references cannot leak preset images into a conversation task", () => {
  const p = fixture();
  const task = createTask(p, [], "image");
  task.prompt = "p";
  for (const plugin of ["gemini-native", "codex-image"]) {
    const field = plugin === "gemini-native" ? "image_urls" : "image";
    const m: MediaModel = {
      ...model("fal-ai/flux/schnell", "image"),
      plugin,
      endpoint:
        plugin === "gemini-native"
          ? "https://generativelanguage.googleapis.com"
          : "codex://local/images",
      params: { model: "codex-image", [field]: ["data:image/png;base64,YQ=="] },
      capabilities: { references: [] },
    };
    expect(inputFor(p, task, m)[field]).toBeUndefined();
    expect(m.params[field]).toEqual(["data:image/png;base64,YQ=="]);
  }
});
