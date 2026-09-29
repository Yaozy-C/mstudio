import { agentLabel } from "../agents/display";
import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { StudioSelect } from "../ui/StudioSelect";
import { AgentTools } from "./AgentTools";
import { createPortal } from "react-dom";
import { ArrowLeft } from "@phosphor-icons/react";
import { useState } from "react";
import { skills, type AgentProfile } from "./catalog";
export function AgentForm({
  initial,
  backTarget,
  save,
  cancel,
  agents,
  switchAgent,
  remember,
}: {
  initial: AgentProfile;
  backTarget: HTMLElement | null;
  agents: AgentProfile[];
  switchAgent: (id: string) => void;
  remember: (profile: AgentProfile) => void;
  save: (p: AgentProfile) => Promise<void>;
  cancel: () => void;
}) {
  useLanguage();
  const [profile, updateProfile] = useState(initial);
  const setProfile = (next: AgentProfile) => {
    updateProfile(next);
    remember(next);
  };
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  function toggle(id: string) {
    setProfile({
      ...profile,
      skillIds: profile.skillIds.includes(id)
        ? profile.skillIds.filter((v) => v !== id)
        : [...profile.skillIds, id],
    });
  }
  return (
    <form
      className="model-form agent-detail-form"
      onSubmit={(e) => {
        e.preventDefault();
        setBusy(true);
        setError("");
        void save(profile)
          .then(cancel)
          .catch((e) => setError(String(e)))
          .finally(() => setBusy(false));
      }}
    >
      {backTarget &&
        createPortal(
          <button
            type="button"
            className="settings-back-button"
            aria-label={t("返回 Agents")}
            title={t("返回 Agents")}
            disabled={busy}
            onClick={cancel}
          >
            <ArrowLeft size={20} />
          </button>,
          backTarget,
        )}
      <div className="agent-detail-navigation">
        <div className="agent-detail-selector">
          <StudioSelect
            label={t("切换 Agent")}
            disabled={busy}
            value={profile.id}
            onValueChange={switchAgent}
            options={[
              ...(!agents.some((a) => a.id === profile.id)
                ? [{ value: profile.id, label: t("新建 Agent") }]
                : []),
              ...agents.map((a) => ({ value: a.id, label: agentLabel(a) })),
            ]}
          />
        </div>
        <button className="primary" disabled={busy}>
          {busy ? t("保存中…") : t("保存 Agent")}
        </button>
      </div>
      <fieldset disabled={busy}>
        <div className="model-form-grid">
          <label>
            {t("名称")}
            <input
              autoFocus
              required
              maxLength={80}
              value={profile.name}
              onChange={(e) => setProfile({ ...profile, name: e.target.value })}
            />
          </label>
          <label>
            {t("职责简介")}
            <input
              maxLength={300}
              value={profile.description}
              onChange={(e) =>
                setProfile({ ...profile, description: e.target.value })
              }
            />
          </label>
          <label className="model-field-wide">
            {t("工作指令")}
            <textarea
              rows={4}
              maxLength={6000}
              value={profile.instructions}
              onChange={(e) =>
                setProfile({ ...profile, instructions: e.target.value })
              }
              placeholder={t("这个 Agent 的目标、工作方式与输出要求…")}
            />
          </label>
        </div>
        <AgentTools profile={profile} onChange={setProfile} />
        <div className="agent-config-block">
          <h3>{t("装配 Skills")}</h3>
          <p className="model-hint">
            {t("Skills 提供专业规则和方法；实际操作权限由工具配置决定。")}
          </p>
          {skills.map((s) => (
            <label className="agent-skill-choice" key={s.id}>
              <input
                type="checkbox"
                checked={profile.skillIds.includes(s.id)}
                onChange={() => toggle(s.id)}
              />
              <span>
                <strong>{t(s.name)}</strong>
                <small>{t(s.description)}</small>
              </span>
              <em>{t(s.kind)}</em>
            </label>
          ))}
        </div>
        {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      </fieldset>
    </form>
  );
}
