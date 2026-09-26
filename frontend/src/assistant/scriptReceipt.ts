import type { Project } from "../model";
import { scriptDuration } from "../creative/timing";
export function scriptReceipt(p: Project, operations: unknown) {
  if (!Array.isArray(operations)) return [];
  const ids = new Set(
    operations
      .filter(
        (o) =>
          o.plan &&
          ["script", "scriptMode", "removeParagraphIds", "paragraphOrder"].some(
            (k) => k in o.plan,
          ),
      )
      .map((o) => o.id),
  );
  return p.nodes
    .filter((n) => ids.has(n.id) && n.kind === "plan")
    .map((n) => ({
      id: n.id,
      title: n.title,
      paragraphCount: n.plan?.script?.length ?? 0,
      duration: scriptDuration(n.plan?.script ?? []),
      paragraphIds: n.plan?.script?.map((s) => s.id) ?? [],
      message: "结构化脚本已写入脚本页面并保存；现有镜头和素材保留",
    }));
}
