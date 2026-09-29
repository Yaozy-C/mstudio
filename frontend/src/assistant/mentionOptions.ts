import type { AgentProfile } from "../agents/catalog";
import type { Project } from "../model";
import { describeAttachment, type AttachmentRef } from "./attachments";
import { matchingAgents } from "./mentions";
export type MentionOption = {
  key: string;
  title: string;
  description: string;
  agent?: AgentProfile;
  ref?: AttachmentRef;
  disabled?: boolean;
};
export function mentionOptions(
  project: Project,
  agents: AgentProfile[],
  query: string,
  symbol: "@" | "$" = "@",
): MentionOption[] {
  const key = query.trim().toLocaleLowerCase();
  if (symbol === "$")
    return matchingAgents(agents, query).map((a) => ({
      key: `agent:${a.id}`,
      title: a.name,
      description: `Agent · ${a.description}`,
      agent: a,
    }));
  const options: MentionOption[] = [];
  const refs: { ref: AttachmentRef; type: string }[] = [
    ...project.nodes.map((n) => ({
      ref: { kind: "node" as const, id: n.id },
      type:
        n.kind === "screenplay"
          ? "脚本 / 方案"
          : n.kind === "shot"
            ? "镜头"
            : n.kind === "asset"
              ? "画布素材"
              : "笔记",
    })),
    ...project.assets.map((a) => ({
      ref: { kind: "asset" as const, id: a.id },
      type: "素材",
    })),
    ...project.clips.map((c) => ({
      ref: { kind: "clip" as const, id: c.id },
      type: "时间线片段",
    })),
  ];
  for (const { ref, type } of refs) {
    const a = describeAttachment(project, ref);
    if (a && `${type} ${a.title}`.toLocaleLowerCase().includes(key))
      options.push({
        key: `${ref.kind}:${ref.id}`,
        title: a.title,
        description: type,
        ref,
      });
  }
  return options.slice(0, 60);
}
