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

// Execution telemetry is persisted, but does not change the creative brief.
// Operations still validate the current task's execution state when applied.
const taskRuntimeFields = [
  "status",
  "progress",
  "error",
  "trackingPaused",
  "jobId",
  "submissionId",
  "requestId",
];

// Layout and task telemetry must not invalidate an Agent's content edit.
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
      "drafts",
    ])
  )
    return true;
  const beforeTasks = before.production?.drafts ?? {};
  const afterTasks = after.production?.drafts ?? {};
  const taskKeys = new Set([
    ...Object.keys(beforeTasks),
    ...Object.keys(afterTasks),
  ]);
  for (const key of taskKeys) {
    if (
      !beforeTasks[key] ||
      !afterTasks[key] ||
      !sameExcept(beforeTasks[key], afterTasks[key], taskRuntimeFields)
    )
      return true;
  }
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
