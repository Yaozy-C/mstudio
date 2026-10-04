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
// Select committed fields recursively. Arrays are complete saved values: their
// merge/removal semantics belong to the editor, never to the receipt builder.
function selected(saved: unknown, requested: unknown): unknown {
  if (!requested || typeof requested !== "object" || Array.isArray(requested))
    return saved ?? null;
  const source = (saved ?? {}) as Record<string, unknown>;
  return Object.fromEntries(
    Object.entries(requested).map(([key, value]) => [
      key,
      selected(source[key], value),
    ]),
  );
}
// Read receipts from the saved document, never echo requested values as proof.
export function savedValues(p: Project, operations: unknown) {
  if (!Array.isArray(operations)) return [];
  return operations
    .filter((o) => o.op === "update_node" || o.op === "add_node")
    .map((o) => {
      const node = p.nodes.find((n) => n.id === o.id);
      const fields = Object.fromEntries(
        Object.entries(o).filter(([key]) => key !== "op" && key !== "id"),
      );
      if (o.screenplay) {
        const commands = ["scriptMode", "removeParagraphIds", "paragraphOrder"];
        fields.screenplay = Object.fromEntries(
          Object.entries(o.screenplay).filter(
            ([key]) => !commands.includes(key),
          ),
        );
        if (commands.some((key) => key in o.screenplay))
          (fields.screenplay as Record<string, unknown>).script = [];
      }
      return {
        id: o.id,
        exists: !!node,
        complete: true,
        values: selected(node, fields),
      };
    });
}
