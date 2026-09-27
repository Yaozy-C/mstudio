import type { Project } from "../model";

function equal(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (!a || !b || typeof a !== "object" || typeof b !== "object") return false;
  if (Array.isArray(a) || Array.isArray(b))
    return (
      Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((v, i) => equal(v, b[i]))
    );
  return sameExcept(a, b, []);
}
function sameExcept<T extends object>(a: T, b: T, ignored: string[]) {
  if (a === b) return true;
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].every(
    (key) =>
      ignored.includes(key) || equal(a[key as keyof T], b[key as keyof T]),
  );
}

// Layout is saved normally, but must not invalidate an Agent's content edit.
export function contentChanged(before: Project, after: Project): boolean {
  if (
    !sameExcept(before, after, [
      "revision",
      "updated",
      "viewport",
      "nodes",
      "production",
    ])
  )
    return true;
  if (
    !sameExcept(before.production ?? {}, after.production ?? {}, [
      "positions",
      "viewport",
    ])
  )
    return true;
  if (before.nodes === after.nodes) return false;
  if (before.nodes.length !== after.nodes.length) return true;
  const nodes = new Map(before.nodes.map((node) => [node.id, node]));
  return after.nodes.some((node) => {
    const previous = nodes.get(node.id);
    return (
      !previous || !sameExcept(previous, node, ["x", "y", "width", "height"])
    );
  });
}
