import {
  describeAttachment,
  type AttachmentRef,
} from "../assistant/attachments";
import type { Project } from "../model";
import type { ProductionInput } from "./types";
export function attachmentInput(
  p: Project,
  ref: AttachmentRef,
): ProductionInput | undefined {
  const a = describeAttachment(p, ref);
  if (!a) return;
  const node =
    ref.kind === "node" ? p.nodes.find((n) => n.id === ref.id) : undefined;
  if (
    node &&
    (node.kind === "shot" ||
      node.kind === "plan" ||
      (!node.assetId && !node.resultAssetId))
  )
    return {
      key: `node:${node.id}`,
      nodeId: node.id,
      assetId: "",
      role: "script",
      purpose: [
        node.title,
        node.text,
        ...(node.plan?.script?.map(
          (s) =>
            `${s.title}\n${s.action}\n${s.onScreenText ? `画面文字：${s.onScreenText}\n` : ""}${s.dialogue}\n${s.sound}`,
        ) ?? []),
      ].join("\n\n"),
    };
  const asset = p.assets.find((v) => v.id === a.assetId);
  if (!asset) return;
  const clip =
    ref.kind === "clip" ? p.clips.find((c) => c.id === ref.id) : undefined;
  return {
    key: clip ? `clip:${clip.id}` : `asset:${asset.id}`,
    sourceRef: ref,
    assetId: asset.id,
    purpose: clip ? a.title : asset.name,
    role:
      asset.kind === "video"
        ? "video-reference"
        : asset.kind === "image"
          ? "reference"
          : "script",
    ...(asset.kind === "video"
      ? {
          start: clip?.trimIn ?? 0,
          end: clip?.trimOut ?? Math.min(5, asset.duration),
        }
      : {}),
  };
}

export function sameInput(a: ProductionInput, b: ProductionInput): boolean {
  if (a.sourceRef?.kind === "clip" || b.sourceRef?.kind === "clip")
    return (
      a.sourceRef?.kind === b.sourceRef?.kind &&
      a.sourceRef?.id === b.sourceRef?.id
    );
  return a.key === b.key || (!!a.assetId && a.assetId === b.assetId);
}
