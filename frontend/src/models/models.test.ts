import { expect, test } from "bun:test";
import {
  localEndpoint,
  readyModel,
  selectedModel,
  type ModelConnection,
} from "./types";
const a: ModelConnection = {
  id: "a",
  name: "A",
  model: "model-a",
  endpoint: "https://models.example/v1",
  adapter: "openai-compatible",
  hasKey: true,
  inputs: { image: false, audio: false, video: false, document: false },
};
const b: ModelConnection = {
  ...a,
  id: "b",
  name: "B",
  model: "model-b",
  inputs: { image: true, audio: false, video: false, document: false },
};
test("project model selection overrides default and keeps that model's image capability", () => {
  const catalog = { profiles: [a, b], defaultId: "a", selectedId: "b" };
  expect(selectedModel(catalog)).toBe(b);
  expect(selectedModel({ ...catalog, selectedId: null })).toBe(a);
  expect(selectedModel({ ...catalog, selectedId: "removed" })).toBeUndefined();
});
test("remote profiles require a credential while loopback can be unauthenticated", () => {
  expect(readyModel({ ...a, hasKey: false })).toBe(false);
  expect(
    readyModel({ ...a, hasKey: false, endpoint: "http://127.0.0.1:1234/v1" }),
  ).toBe(true);
  expect(localEndpoint("http://localhost.evil.example/v1")).toBe(false);
  expect(localEndpoint("http://[::1]:1234/v1")).toBe(true);
});
