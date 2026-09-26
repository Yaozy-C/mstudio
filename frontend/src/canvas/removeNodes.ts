import type { Project } from "../model";
export function removeNodes(p: Project, ids: string[]): Project {
  const removed = new Set(ids);
  for (const node of p.nodes)
    if (node.shot && removed.has(node.shot.planId)) removed.add(node.id);
  return {
    ...p,
    nodes: p.nodes.filter((n) => !removed.has(n.id)),
    clips: p.clips.map((c) =>
      c.shotId && removed.has(c.shotId) ? { ...c, shotId: undefined } : c,
    ),
  };
}
