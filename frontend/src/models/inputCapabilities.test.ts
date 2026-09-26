import { expect, test } from "bun:test";
import { newConnection } from "./types";
import { supportsInput } from "./inputCapabilities";
import { unsupportedAttachments } from "../assistant/AttachmentSupport";
import { appendAsset } from "../timeline/document";
import { newProject, type Asset } from "../model";
test("protocol gates declared capabilities and reports unsupported references", () => {
  const model = {
    ...newConnection(),
    inputs: { image: true, audio: true, video: true, document: true },
  };
  expect(supportsInput(model, "audio")).toBe(true);
  expect(supportsInput(model, "video")).toBe(false);
  expect(supportsInput({ ...model, adapter: "gemini-native" }, "video")).toBe(
    true,
  );
  expect(
    supportsInput({ ...model, adapter: "anthropic-native" }, "audio"),
  ).toBe(false);
  expect(
    supportsInput({ ...model, adapter: "anthropic-native" }, "document"),
  ).toBe(true);
  expect(
    supportsInput({ ...model, adapter: "openai-responses" }, "document"),
  ).toBe(true);
  expect(
    supportsInput({ ...model, adapter: "openai-responses" }, "audio"),
  ).toBe(false);
  const refs = [
    { kind: "asset" as const, id: "v", title: "video", mediaKind: "video" },
  ];
  expect(unsupportedAttachments(refs, model)).toHaveLength(1);
});
test("reference documents never enter timeline", () => {
  const project = newProject("test");
  for (const kind of ["text", "document"] as const) {
    const asset: Asset = {
      id: "a",
      kind,
      name: "brief",
      path: "",
      preview: "",
      duration: 0,
      width: 0,
      height: 0,
      hasAudio: false,
    };
    expect(appendAsset(project, asset)).toBe(project);
  }
});
