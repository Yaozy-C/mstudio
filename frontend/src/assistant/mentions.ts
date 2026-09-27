import { agentLabel } from "../agents/display";
import type { AgentProfile } from "../agents/catalog";
import { selectedModel, type ModelCatalog } from "../models/types";
export function mentionQuery(text: string, caret: number) {
  const prefix = text.slice(0, caret);
  const match = /(?:^|\s)([@$])([^@$\n]{0,60})$/u.exec(prefix);
  return match
    ? {
        start: prefix.lastIndexOf(match[1]),
        end: caret,
        query: match[2],
        symbol: match[1] as "@" | "$",
      }
    : null;
}
export function matchingAgents(agents: AgentProfile[], query: string) {
  const key = query.trim().toLocaleLowerCase();
  return agents.filter(
    (a) =>
      a.enabled &&
      `${a.name} ${a.description} ${agentLabel(a)} ${agentLabel(a, "description")}`
        .toLocaleLowerCase()
        .includes(key),
  );
}
export function resolveMention(
  agents: AgentProfile[],
  catalog: ModelCatalog,
  id: string | null,
) {
  const agent = agents.find((a) => a.id === (id || "coordinator"));
  if (!agent?.enabled)
    throw new Error(
      id ? "选择的 Agent 已移除或停用，请重新选择" : "请在设置中启用默认助手",
    );
  const model = selectedModel(catalog);
  if (!model) throw new Error("请为当前对话选择文本模型");
  return { agent, model };
}
