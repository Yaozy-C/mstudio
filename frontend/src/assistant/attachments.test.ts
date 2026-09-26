import { test, expect } from "bun:test";
import { newProject, type Asset } from "../model";
import {
  attachmentKey,
  describeAttachment,
  historyAttachments,
  mergeAttachments,
  type AttachmentRef,
} from "./attachments";
test("explicit attachments retain identities across selection changes and deduplicate only the same reference", () => {
  const refs: AttachmentRef[] = [
    { kind: "node", id: "shot" },
    { kind: "asset", id: "image" },
  ];
  expect(mergeAttachments(refs, [refs[0]])).toEqual(refs);
  const full = mergeAttachments(
    refs,
    Array.from({ length: 14 }, (_, i) => i).map((i) => ({
      kind: "asset",
      id: String(i),
    })),
  );
  expect(full.length).toBe(12);
  expect(full.slice(0, 2)).toEqual(refs);
  expect(attachmentKey({ kind: "node", id: "same" })).not.toBe(
    attachmentKey({ kind: "asset", id: "same" }),
  );
});
test("attachments identify the chosen result, preserve historical media and report deleted objects", () => {
  const p = newProject("attachments");
  const asset = (id: string): Asset => ({
    id,
    name: id,
    path: "/local",
    preview: "/preview",
    kind: "image",
    duration: 0,
    width: 10,
    height: 10,
    hasAudio: false,
  });
  p.assets = [asset("reference"), asset("result")];
  p.nodes = [
    {
      id: "shot",
      kind: "shot",
      title: "镜头",
      text: "动作",
      x: 0,
      y: 0,
      assetId: "reference",
      resultAssetId: "result",
    },
  ];
  const ref = { kind: "node" as const, id: "shot" };
  const snapshot = describeAttachment(p, ref)!;
  expect(snapshot.assetId).toBeUndefined();
  p.nodes[0].resultAssetId = "reference";
  const history = historyAttachments([
    { type: "text", text: "request", attachments: [snapshot] },
  ]);
  expect(history[0].assetId).toBeUndefined();
  p.nodes = [];
  expect(describeAttachment(p, ref)).toBeNull();
  expect(historyAttachments("plain text message")).toEqual([]);
  expect(
    historyAttachments([
      { attachments: [{ kind: "invalid", id: "x", title: "x" }] },
    ]),
  ).toEqual([]);
});
