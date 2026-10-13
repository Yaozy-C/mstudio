import type { Project } from "../model";
/** Guard all structured Agent writes, including canvas, frames and timeline inputs. */
export function assertUsableAgentReferences(p: Project, value: unknown) {
  if (!value || typeof value !== "object") return;
  if (Array.isArray(value)) {
    value.forEach((v) => assertUsableAgentReferences(p, v));
    return;
  }
  for (const [key, field] of Object.entries(value)) {
    if (
      ["assetId", "resultAssetId"].includes(key) &&
      typeof field === "string" &&
      p.assets.some((a) => a.id === field && a.discarded)
    )
      throw new Error(`素材 ${field} 已废弃，不能被 Agent 引用`);
    assertUsableAgentReferences(p, field);
  }
}
