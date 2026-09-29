import type { BoardNode, Project } from "../model";

export function promptBasis(p: Project, node: BoardNode) {
  const screenplay = p.nodes.find((n) => n.id === node.shot?.screenplayId);
  const source = screenplay?.screenplay?.script?.find(
    (s) => s.id === node.shot?.scriptId,
  );
  return JSON.stringify([
    node.text,
    node.shot?.dialogue,
    node.shot?.duration,
    node.shot?.frames?.map((f) => [f.assetId, f.prompt]),
    node.references,
    screenplay?.title,
    p.creation?.intent,
    p.creation?.essential,
    p.creation?.preserve,
    p.width,
    p.height,
    ...(source ? [source] : []),
  ]);
}
export const promptStale = (p: Project, node: BoardNode) =>
  !!node.shot?.prompt && node.shot.promptBasis !== promptBasis(p, node);
