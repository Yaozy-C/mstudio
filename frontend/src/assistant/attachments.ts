import { duration, type Project } from "../model";
export type AttachmentRef = { kind: "node" | "asset" | "clip"; id: string };
export type Attachment = AttachmentRef & {
  title: string;
  assetId?: string;
  mediaKind?: string;
};
export const MAX_ATTACHMENTS = 12;
export const attachmentKey = (a: AttachmentRef) => `${a.kind}:${a.id}`;
export function mergeAttachments(
  current: AttachmentRef[],
  added: AttachmentRef[],
) {
  const unique = new Map(current.map((a) => [attachmentKey(a), a]));
  for (const a of added) unique.set(attachmentKey(a), a);
  return [...unique.values()].slice(0, MAX_ATTACHMENTS);
}
export function describeAttachment(
  project: Project,
  ref: AttachmentRef,
): Attachment | null {
  const clip =
    ref.kind === "clip"
      ? project.clips.find((c) => c.id === ref.id)
      : undefined;
  if (ref.kind === "clip" && !clip) return null;
  const node =
    ref.kind === "node"
      ? project.nodes.find((n) => n.id === ref.id)
      : undefined;
  const assetId =
    ref.kind === "asset"
      ? ref.id
      : clip?.assetId ||
        (node?.kind === "shot" || node?.kind === "screenplay"
          ? undefined
          : node?.resultAssetId || node?.assetId);
  const asset = project.assets.find((a) => a.id === assetId);
  if (ref.kind === "node" ? !node : !asset) return null;
  return {
    ...ref,
    title: clip
      ? `${+(clip.start ?? 0).toFixed(2)}–${+((clip.start ?? 0) + duration(clip)).toFixed(2)}s · ${asset?.name ?? "片段"}`
      : node?.title || asset?.name || "未命名",
    ...(asset ? { assetId: asset.id, mediaKind: asset.kind } : {}),
  };
}
export function historyAttachments(payload: unknown): Attachment[] {
  const value = Array.isArray(payload) ? payload[0]?.attachments : null;
  return Array.isArray(value)
    ? value
        .filter(
          (a) =>
            a &&
            ["node", "asset", "clip"].includes(a.kind) &&
            typeof a.id === "string" &&
            typeof a.title === "string",
        )
        .slice(0, MAX_ATTACHMENTS)
    : [];
}
