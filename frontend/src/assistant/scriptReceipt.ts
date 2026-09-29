import type { Project } from "../model";
import { scriptDuration } from "../creative/timing";
export function scriptReceipt(p: Project, operations: unknown) {
  if (!Array.isArray(operations)) return [];
  const ids = new Set(
    operations
      .filter(
        (o) =>
          o.screenplay &&
          ["script", "scriptMode", "removeParagraphIds", "paragraphOrder"].some(
            (k) => k in o.screenplay,
          ),
      )
      .map((o) => o.id),
  );
  return p.nodes
    .filter((n) => ids.has(n.id) && n.kind === "screenplay")
    .map((n) => ({
      id: n.id,
      title: n.title,
      paragraphCount: n.screenplay?.script?.length ?? 0,
      duration: scriptDuration(n.screenplay?.script ?? []),
      paragraphIds: n.screenplay?.script?.map((s) => s.id) ?? [],
      message:
        "Structured script saved to the screenplay; existing shots and media retained.",
    }));
}
