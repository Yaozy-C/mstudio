import type { Project } from "../model";
export type WorkContext = {
  view: "script" | "storyboard" | "film";
  planId?: string;
  paragraphId?: string;
};
// Workspace focus selects the object, never the user's conversational role.
export function defaultAgent(_work?: WorkContext) {
  return "coordinator";
}
export function workTarget(project: Project, work?: WorkContext) {
  if (work?.view !== "script" || !work.planId) return null;
  const plan = project.nodes.find(
    (n) => n.id === work.planId && n.kind === "plan",
  );
  if (!plan) throw new Error("当前脚本已移除，请重新选择");
  if (
    work.paragraphId &&
    !plan.plan?.script?.some((s) => s.id === work.paragraphId)
  )
    throw new Error("当前段落已移除，请重新选择");
  return plan.id;
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
    .filter((n) => n?.kind === "plan" || n?.kind === "shot");
  const target =
    explicit ?? (nodes.length === 1 ? nodes[0]!.id : workTarget(project, work));
  return { target, agent: defaultAgent() };
}
