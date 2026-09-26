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
      <div className="hub-section-head">
        <div className="roster-heading">
          <h2>我的 Agents</h2>
          <p>在聊天中 @ 你的创作团队。</p>
        </div>
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
              toolIds: ["project-read", "memory-read"],
              enabled: true,
              revision: 0,
            })
          }
        >
          <Plus />
          创建 Agent
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
                {a.name}
                {a.id === "coordinator" && <small>默认助手</small>}
              </h3>
              <p>{a.description || "尚未填写职责"}</p>
            </div>
            <span className="agent-roster-count">
              {a.skillIds.length} 个 Skills <span> / </span> {a.toolIds.length}{" "}
              个工具
            </span>
            <label className="agent-roster-toggle">
              <Switch
                size="2"
                checked={a.enabled}
                disabled={!native || busy}
                onCheckedChange={() => void toggle(a)}
                aria-label={`${a.enabled ? "停用" : "启用"} ${a.name}`}
              />
              <span>{a.enabled ? "已启用" : "已停用"}</span>
            </label>
            <button
              disabled={!native || busy}
              onClick={() => setEdit(drafts.current.get(a.id) ?? a)}
              aria-label={`配置 ${a.name}`}
            >
              <PencilSimple />
              编辑
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
