import { paragraphDuration } from "./timing";
import { uid, type BoardNode, type Project } from "../model";
import type { ScriptParagraph } from "./types";
import { shotsOf } from "./document";

export const paragraphBasis = (s: ScriptParagraph) =>
  JSON.stringify([
    s.title,
    s.action,
    s.dialogue,
    s.sound,
    ...(s.duration === undefined ? [] : [s.duration]),
    ...(s.onScreenText ? [{ onScreenText: s.onScreenText }] : []),
  ]);
export const newParagraph = (): ScriptParagraph => ({
  id: uid(),
  duration: 5,
  title: "",
  action: "",
  dialogue: "",
  sound: "",
});
export function scriptSource(p: Project, n: BoardNode) {
  return p.nodes
    .find((v) => v.id === n.shot?.screenplayId)
    ?.screenplay?.script?.find((s) => s.id === n.shot?.scriptId);
}
export function scriptChanged(p: Project, n: BoardNode) {
  const source = scriptSource(p, n);
  return source
    ? n.shot?.scriptBasis !== paragraphBasis(source)
    : !!n.shot?.scriptId;
}
export function createScript(p: Project): Project {
  if (p.nodes.some((n) => n.kind === "screenplay")) return p;
  return {
    ...p,
    nodes: [
      ...p.nodes,
      {
        id: uid(),
        kind: "screenplay",
        title: "新脚本",
        text: "",
        x: 80,
        y: 80,
        screenplay: { script: [newParagraph()] },
      },
    ],
  };
}
export function updateParagraph(
  p: Project,
  screenplayId: string,
  id: string,
  patch: Partial<ScriptParagraph>,
): Project {
  return {
    ...p,
    nodes: p.nodes.map((n) =>
      n.id !== screenplayId
        ? n
        : {
            ...n,
            screenplay: {
              ...n.screenplay!,
              script: n.screenplay!.script!.map((s) =>
                s.id === id ? { ...s, ...patch, id: s.id } : s,
              ),
            },
          },
    ),
  };
}
export function splitParagraph(
  p: Project,
  screenplayId: string,
  id: string,
  count: number,
): Project {
  const source = p.nodes
    .find((n) => n.id === screenplayId)
    ?.screenplay?.script?.find((s) => s.id === id);
  if (
    !source ||
    !(source.action.trim() || source.dialogue.trim()) ||
    !Number.isInteger(count) ||
    count < 1 ||
    count > 8
  )
    return p;
  const order = Math.max(
    0,
    ...shotsOf(p, screenplayId).map((n) => n.shot!.order),
  );
  const nodes: BoardNode[] = Array.from({ length: count }, (_, i) => ({
    id: uid(),
    kind: "shot",
    x: 80 + i * 320,
    y: 400,
    title: `${source.title || "段落"} · 镜头 ${i + 1}`,
    text: source.action,
    shot: {
      screenplayId,
      order: order + i + 1,
      duration: paragraphDuration(source) / count,
      dialogue: count === 1 ? source.dialogue : "",
      scriptId: id,
      scriptBasis: paragraphBasis(source),
    },
  }));
  return { ...p, nodes: [...p.nodes, ...nodes] };
}
