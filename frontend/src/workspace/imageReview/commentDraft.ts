import type { ImageComment } from "./annotations";

export const commentDraftKey = (projectId: string, assetId: string) =>
  `mstudio-image-comments:${projectId}:${assetId}`;
export function readCommentDraft(key: string): ImageComment[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) || "[]");
    if (!Array.isArray(value)) return [];
    return value.filter(
      (c): c is ImageComment =>
        c &&
        typeof c.id === "string" &&
        typeof c.text === "string" &&
        [c.x, c.y, c.width, c.height].every(
          (v) => typeof v === "number" && Number.isFinite(v),
        ) &&
        c.x >= 0 &&
        c.y >= 0 &&
        c.width > 0 &&
        c.height > 0 &&
        c.x + c.width <= 1.000001 &&
        c.y + c.height <= 1.000001,
    );
  } catch {
    return [];
  }
}
export function saveCommentDraft(key: string, comments: ImageComment[]) {
  try {
    if (comments.length) localStorage.setItem(key, JSON.stringify(comments));
    else localStorage.removeItem(key);
  } catch {
    /* The open editor still retains the draft when storage is full. */
  }
}
