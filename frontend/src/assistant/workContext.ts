import type { Project } from "../model";
export type WorkContext = {
  view: "script" | "storyboard" | "film";
  screenplayId?: string;
  paragraphId?: string;
};
// Workspace focus selects the object, never the user's conversational role.
export function defaultAgent(_work?: WorkContext) {
  return "coordinator";
}
export function workTarget(project: Project, work?: WorkContext) {
  if (work?.view !== "script" || !work.screenplayId) return null;
  const screenplay = project.nodes.find(
    (n) => n.id === work.screenplayId && n.kind === "screenplay",
  );
  if (!screenplay) throw new Error("当前脚本已移除，请重新选择");
  if (
    work.paragraphId &&
    !screenplay.screenplay?.script?.some((s) => s.id === work.paragraphId)
  )
    throw new Error("当前段落已移除，请重新选择");
  return screenplay.id;
}
export function taskFocus(
  project: Project,
  work: WorkContext | undefined,
  explicit: string | null,
  refs: { kind: string; id: string }[],
  _media: boolean,
) {
  const nodes = refs
    .filter((r) => r.kind === "node")
    .map((r) => project.nodes.find((n) => n.id === r.id))
    .filter((n) => n?.kind === "screenplay" || n?.kind === "shot");
  const target =
    explicit ?? (nodes.length === 1 ? nodes[0]!.id : workTarget(project, work));
  return { target, agent: defaultAgent() };
}
