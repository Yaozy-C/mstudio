import type { AttachmentRef } from "../assistant/attachments";
import type { Project } from "../model";
import { attachmentInput } from "./attachmentInput";
import type { ProductionItem, ProductionTask } from "./types";

export function referencedItems(
  project: Project,
  items: ProductionItem[],
  task: ProductionTask | undefined,
  attachments: AttachmentRef[],
): Set<string> {
  const inputs = task
    ? task.inputs
    : attachments.map((ref) => attachmentInput(project, ref));
  const keys = new Set<string>();
  for (const input of inputs) {
    if (!input) continue;
    const exact = items.find((item) => item.key === input.key);
    if (exact) {
      keys.add(exact.key);
      continue;
    }
    for (const item of items) {
      if (
        input.assetId
          ? item.assetId === input.assetId
          : !item.assetId && item.nodeId === input.nodeId
      )
        keys.add(item.key);
    }
  }
  return keys;
}
