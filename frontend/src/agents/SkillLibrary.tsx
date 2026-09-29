import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { skills, useAgents } from "./catalog";
import { SkillRules } from "./SkillRules";
import type { CreativeSkill } from "../plugins/skills";
export function SkillLibrary({ openAgents }: { openAgents: () => void }) {
  useLanguage();
  const hub = useAgents();
  const [installed, setInstalled] = useState<CreativeSkill[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    if (native)
      void bridge<CreativeSkill[]>("creative_skills")
        .then(setInstalled)
        .catch((e) => setError(String(e)));
  }, []);
  return (
    <section>
      <div className="settings-actions">
        <button onClick={openAgents}>{t("去 Agent 装配")}</button>
      </div>
      {skills.map((s) => (
        <article className="hub-feature" key={s.id}>
          <div className="hub-section-head">
            <div>
              <h3>{t(s.name)}</h3>
              <p>{t(s.description)}</p>
            </div>
            <span className="model-badge">{t(s.kind)}</span>
          </div>
          <p className="model-hint">
            {hub.agents.filter((a) => a.skillIds.includes(s.id)).length}{" "}
            {t("个 Agent 已装配 ·")}{" "}
            {installed.find((v) => v.id === s.id)?.available
              ? t("规则可用")
              : t("规则待检查")}
          </p>
          <SkillRules
            id={s.id}
            available={!!installed.find((v) => v.id === s.id)?.available}
          />
        </article>
      ))}
      {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
    </section>
  );
}
