import { Wrench } from "@phosphor-icons/react";
import { agentLabel } from "../agents/display";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useAgents } from "./catalog";
import { tools } from "./tools";
export function ToolLibrary({ openAgents }: { openAgents: () => void }) {
  useLanguage();
  const hub = useAgents();
  return (
    <section>
      <div className="settings-actions">
        <button className="primary" onClick={openAgents}>
          <Wrench aria-hidden="true" />
          {t("配置工具")}
        </button>
      </div>
      {tools.map((tool) => {
        const assigned = hub.agents.filter((a) => a.toolIds.includes(tool.id));
        return (
          <article className="hub-feature" key={tool.id}>
            <div className="hub-section-head">
              <div>
                <h3>{t(tool.name)}</h3>
                <p>{t(tool.description)}</p>
              </div>
              <span className="model-badge">{t(tool.kind)}</span>
            </div>
            <p className="model-hint">
              {t("已配置给：")}
              {assigned.map((a) => agentLabel(a)).join("、") || t("暂无 Agent")}
            </p>
          </article>
        );
      })}
      {hub.error && (
        <ErrorNotice error={hub.error} fallback="OPERATION_FAILED" />
      )}
    </section>
  );
}
