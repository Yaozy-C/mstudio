import type { Clip, Project } from "../model";

/** Actual post-save deltas include ripple neighbours and invalidated transitions. */
export function clipReceipt(before: Project, after: Project) {
  const previous = new Map(before.clips.map((c) => [c.id, c]));
  const current = new Map(after.clips.map((c) => [c.id, c]));
  const items = [...new Set([...previous.keys(), ...current.keys()])].flatMap(
    (id) => {
      const old = previous.get(id),
        saved = current.get(id);
      if (JSON.stringify(old) === JSON.stringify(saved)) return [];
      const keys = new Set([
        ...Object.keys(old ?? {}),
        ...Object.keys(saved ?? {}),
      ]);
      const values = Object.fromEntries(
        [...keys]
          .filter(
            (key) =>
              key !== "id" &&
              JSON.stringify(old?.[key as keyof Clip]) !==
                JSON.stringify(saved?.[key as keyof Clip]),
          )
          .map((key) => [key, saved?.[key as keyof Clip] ?? null]),
      );
      return [{ id, exists: !!saved, values }];
    },
  );
  return {
    items: items.slice(0, 12),
    total: items.length,
    complete: items.length <= 12,
  };
}
// Read receipts from the saved document, never echo requested values as proof.
export function savedValues(p: Project, operations: unknown) {
  if (!Array.isArray(operations)) return [];
  return operations
    .filter((o) => o.op === "update_node" || o.op === "add_node")
    .map((o) => {
      const node = p.nodes.find((n) => n.id === o.id);
      const paragraphs = Array.isArray(o.screenplay?.script)
        ? o.screenplay.script
        : [];
      return {
        id: o.id,
        shot:
          node?.shot && o.shot
            ? Object.fromEntries(
                ["order", "duration", "screenplayId", "scriptId"]
                  .filter((k) => k in o.shot)
                  .map((k) => [k, node.shot![k as keyof typeof node.shot]]),
              )
            : undefined,
        script: paragraphs.map((patch: { id: string; duration?: number }) => {
          const saved = node?.screenplay?.script?.find(
            (s) => s.id === patch.id,
          );
          return {
            id: patch.id,
            exists: !!saved,
            duration:
              patch.duration !== undefined ? saved?.duration : undefined,
          };
        }),
      };
    })
    .filter((o) => o.shot || o.script.length);
}
