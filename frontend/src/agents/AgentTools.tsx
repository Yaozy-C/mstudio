import { t, useLanguage } from "../i18n";
import type { AgentProfile } from "./catalog";
import { tools } from "./tools";
export function AgentTools({
  profile,
  onChange,
}: {
  profile: AgentProfile;
  onChange: (profile: AgentProfile) => void;
}) {
  useLanguage();
  function toggle(id: string) {
    const selected = new Set(profile.toolIds);
    if (selected.has(id)) {
      selected.delete(id);
      if (id === "project-read")
        for (const tool of selected)
          if (tool.startsWith("project-")) selected.delete(tool);
    } else {
      selected.add(id);
      if (id.startsWith("project-") && id !== "project-read")
        selected.add("project-read");
    }
    onChange({ ...profile, toolIds: [...selected] });
  }
  return (
    <div className="agent-config-block">
      <h3>{t("工具权限")}</h3>
      <p className="model-hint">
        {t("只向这个 Agent 开放勾选的操作。工具权限与 Skills 分开配置。")}
      </p>
      {tools.map((tool) => (
        <label className="agent-skill-choice" key={tool.id}>
          <input
            type="checkbox"
            checked={profile.toolIds.includes(tool.id)}
            onChange={() => toggle(tool.id)}
          />
          <span>
            <strong>{t(tool.name)}</strong>
            <small>{t(tool.description)}</small>
          </span>
          <em>{t(tool.kind)}</em>
        </label>
      ))}
    </div>
  );
}
