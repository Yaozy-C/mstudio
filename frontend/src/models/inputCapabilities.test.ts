import { expect, test } from "bun:test";
import { newConnection } from "./types";
import { supportsInput } from "./inputCapabilities";
import { unsupportedAttachments } from "../assistant/AttachmentSupport";
import { appendAsset } from "../timeline/document";
import { newProject, type Asset } from "../model";
test("configured inputs are honored regardless of protocol or model", () => {
  const model = {
    ...newConnection(),
    inputs: { image: true, audio: true, video: true, document: true },
  };
  for (const adapter of [
    "openai-compatible",
    "gemini-native",
    "anthropic-native",
    "openai-responses",
    "codex",
  ] as const) {
    for (const kind of ["image", "audio", "video", "document"] as const) {
      expect(supportsInput({ ...model, adapter }, kind)).toBe(true);
      expect(
        supportsInput(
          { ...model, adapter, inputs: { ...model.inputs, [kind]: false } },
          kind,
        ),
      ).toBe(false);
    }
  }
  const refs = [
    { kind: "asset" as const, id: "v", title: "video", mediaKind: "video" },
  ];
  expect(
    unsupportedAttachments(refs, {
      ...model,
      inputs: { ...model.inputs, image: false },
    }),
  ).toHaveLength(0);
  expect(
    unsupportedAttachments(refs, {
      ...model,
      inputs: { ...model.inputs, image: false, video: false },
    }),
  ).toHaveLength(1);
  expect(
    unsupportedAttachments(refs, {
      ...model,
      inputs: { ...model.inputs, video: false },
    }),
  ).toHaveLength(0);
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
