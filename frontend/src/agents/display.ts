import builtins from "./defaults.json";
import type { AgentProfile } from "./catalog";
import { t } from "../i18n";

/** Localize built-in labels only while their original values are unchanged. */
export function agentLabel(
  agent: Pick<AgentProfile, "id" | "name" | "description">,
  field: "name" | "description" = "name",
) {
  const original = builtins.find((item) => item.id === agent.id);
  return original?.[field] === agent[field] ? t(agent[field]) : agent[field];
}
