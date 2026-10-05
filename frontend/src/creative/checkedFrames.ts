import type { Project } from "../model";
import type { Frame } from "../production/types";

export function checkedFrames(
  p: Project,
  raw: unknown,
  existing: Frame[] = [],
): Frame[] {
  if (!Array.isArray(raw) || raw.length > 50)
    throw new Error("每个镜头最多 50 张分镜图");
  const frames = raw.map((value) => {
    if (!value || typeof value !== "object" || Array.isArray(value))
      throw new Error("分镜图格式无效");
    if (
      Object.keys(value).some(
        (key) => !["assetId", "title", "prompt"].includes(key),
      )
    )
      throw new Error("分镜图仅支持 assetId、title、prompt");
    const { assetId, title, prompt } = value;
    if (
      p.assets.some((a) => a.id === assetId && a.inLibrary === false) &&
      !existing.some((f) => f.assetId === assetId)
    )
      throw new Error("素材已从素材库移除，不能新增引用；请先恢复素材");
    if (
      typeof assetId !== "string" ||
      !p.assets.some((a) => a.id === assetId && a.kind === "image")
    )
      throw new Error("分镜图必须是本项目的图片");
    if (
      typeof title !== "string" ||
      title.length > 300 ||
      (prompt !== undefined &&
        (typeof prompt !== "string" || prompt.length > 12000))
    )
      throw new Error("分镜图标题或描述无效");
    return {
      assetId,
      title,
      prompt,
    };
  });
  if (new Set(frames.map((f) => f.assetId)).size !== frames.length)
    throw new Error("同一镜头不能重复关联同一张分镜图");
  return frames;
}
