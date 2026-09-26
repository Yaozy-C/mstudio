import { useAgents } from "../agents/catalog";
import { useMediaModels } from "../models/mediaRegistry";
export function useGenerationAgent() {
  const media = useMediaModels();
  const agent = useAgents().agents.find((a) => a.id === "production");
  const models =
    agent?.enabled && agent.toolIds.includes("media-generation")
      ? media.models.filter((m) => m.enabled)
      : [];
  return { ...media, models, agent };
}
