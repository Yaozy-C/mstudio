import type { AgentProfile } from "./catalog";
import { tools } from "./tools";
export function AgentTools({
  profile,
  onChange,
}: {
  profile: AgentProfile;
  onChange: (profile: AgentProfile) => void;
}) {
  function toggle(id: string) {
    const selected = new Set(profile.toolIds);
    if (selected.has(id)) {
      selected.delete(id);
      if (id === "project-read")
        for (const tool of selected)
          if (tool.startsWith("project-")) selected.delete(tool);
      if (id === "memory-read") selected.delete("memory-write");
    } else {
      selected.add(id);
      if (id.startsWith("project-") && id !== "project-read")
        selected.add("project-read");
      if (id === "memory-write") selected.add("memory-read");
    }
    onChange({ ...profile, toolIds: [...selected] });
  }
  return (
    <div className="agent-config-block">
      <h3>工具权限</h3>
      <p className="model-hint">
        只向这个 Agent 开放勾选的操作。工具权限与 Skills 分开配置。
      </p>
      {tools.map((tool) => (
        <label className="agent-skill-choice" key={tool.id}>
          <input
            type="checkbox"
            checked={profile.toolIds.includes(tool.id)}
            onChange={() => toggle(tool.id)}
          />
          <span>
            <strong>{tool.name}</strong>
            <small>{tool.description}</small>
          </span>
          <em>{tool.kind}</em>
        </label>
      ))}
    </div>
  );
}
