import { agentLabel } from "../agents/display";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useLayoutEffect, useRef, useState } from "react";
import { AgentMark } from "../ui/Identity";
import { Switch } from "@radix-ui/themes";
import { Plus, PencilSimple } from "@phosphor-icons/react";
import { native } from "../bridge";
import { useAgents, type AgentProfile } from "../agents/catalog";
import { AgentForm } from "../agents/AgentForm";
export function AgentCenter({
  onNavigate,
  backTarget,
}: {
  onNavigate: () => void;
  backTarget: HTMLElement | null;
}) {
  useLanguage();
  const hub = useAgents();
  const [edit, setEdit] = useState<AgentProfile | null>(null);
  useLayoutEffect(onNavigate, [edit?.id, onNavigate]);
  const drafts = useRef(new Map<string, AgentProfile>());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function toggle(agent: AgentProfile) {
    setBusy(true);
    setError("");
    try {
      await hub.save({ ...agent, enabled: !agent.enabled });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  if (edit)
    return (
      <AgentForm
        key={edit.id}
        initial={edit}
        backTarget={backTarget}
        save={async (profile) => {
          await hub.save(profile);
          drafts.current.delete(profile.id);
        }}
        agents={hub.agents}
        switchAgent={(id) =>
          setEdit(
            drafts.current.get(id) ??
              hub.agents.find((a) => a.id === id) ??
              null,
          )
        }
        remember={(profile) => drafts.current.set(profile.id, profile)}
        cancel={() => setEdit(null)}
      />
    );
  return (
    <section>
      <div className="settings-actions">
        <button
          className="primary"
          disabled={!native || hub.loading}
          onClick={() =>
            setEdit({
              id: crypto.randomUUID(),
              name: "",
              description: "",
              instructions: "",
              skillIds: [],
              toolIds: ["project-read"],
              enabled: true,
              revision: 0,
            })
          }
        >
          <Plus />
          {t("创建 Agent")}
        </button>
      </div>
      <div className="agent-roster">
        {hub.agents.map((a) => (
          <article className="agent-roster-row" key={a.id}>
            <div className="agent-roster-icon">
              <AgentMark id={a.id} size={28} />
            </div>
            <div className="agent-roster-info">
              <h3>
                {agentLabel(a)}
                {a.id === "coordinator" && <small>{t("默认助手")}</small>}
              </h3>
              <p>{agentLabel(a, "description") || t("尚未填写职责")}</p>
            </div>
            <span className="agent-roster-count">
              {a.skillIds.length} {t("个 Skills")} <span> / </span>{" "}
              {a.toolIds.length} {t("个工具")}
            </span>
            <label className="agent-roster-toggle">
              <Switch
                size="2"
                checked={a.enabled}
                disabled={!native || busy}
                onCheckedChange={() => void toggle(a)}
                aria-label={`${a.enabled ? t("停用") : t("启用")} ${agentLabel(a)}`}
              />
              <span>{a.enabled ? t("已启用") : t("已停用")}</span>
            </label>
            <button
              disabled={!native || busy}
              onClick={() => setEdit(drafts.current.get(a.id) ?? a)}
              aria-label={t("配置 {v0}", { v0: agentLabel(a) })}
            >
              <PencilSimple />
              {t("编辑")}
            </button>
          </article>
        ))}
      </div>
      {(error || hub.error) && (
        <ErrorNotice error={error || hub.error} fallback="OPERATION_FAILED" />
      )}
    </section>
  );
}
