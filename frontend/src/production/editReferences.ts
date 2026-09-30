import type { Project, Reference } from "../model";
import { creationOperation } from "../assistant/creationOperations";

// Apply only the user's local changes to the latest project, preserving agent
// additions made while the editor was open.
export function editReferences(
  p: Project,
  id: string,
  before: Reference[],
  after: Reference[],
): Project {
  const removed = before
    .filter((r) => !after.some((next) => next.assetId === r.assetId))
    .map((r) => r.assetId);
  const changed = after.filter(
    (r) =>
      JSON.stringify(r) !==
      JSON.stringify(before.find((old) => old.assetId === r.assetId)),
  );
  let next = p;
  if (removed.length)
    next = creationOperation(next, {
      op: "set_references",
      id,
      referenceMode: "remove",
      assetIds: removed,
    })!;
  if (changed.length)
    next = creationOperation(next, {
      op: "set_references",
      id,
      referenceMode: "upsert",
      references: changed,
    })!;
  return next;
}
