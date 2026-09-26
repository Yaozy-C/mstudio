import type { BoardNode, Project } from "../model";
import { creativeExtras } from "./operations";

export type ShotTextField = "action" | "dialogue";

export const actionText = (node: BoardNode) => node.text;

export function editShotText(
  project: Project,
  id: string,
  field: ShotTextField,
  value: string,
): Project {
  const node = project.nodes.find((n) => n.id === id && n.kind === "shot");
  if (!node?.shot) return project;
  const old = field === "action" ? actionText(node) : node.shot.dialogue;
  if (value === old) return project;
  const updated = creativeExtras(
    project,
    {
      ...node,
      text: field === "action" ? value : node.text,
      shot: {
        ...node.shot,
        dialogue: field === "dialogue" ? value : node.shot.dialogue,
      },
    },
    {},
  );
  return {
    ...project,
    nodes: project.nodes.map((n) => (n.id === id ? updated : n)),
  };
}
