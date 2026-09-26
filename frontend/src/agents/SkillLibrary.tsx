import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { bridge, native } from "../bridge";
import { skills, useAgents } from "./catalog";
import { SkillRules } from "./SkillRules";
import type { CreativeSkill } from "../plugins/skills";
export function SkillLibrary({ openAgents }: { openAgents: () => void }) {
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
      <div className="hub-intro">
        <div className="eyebrow">SKILLS · 能力层</div>
        <h2>把专业方法与规则装配给 Agent。</h2>
        <p>
          Skill 库包含随 Mstudio 发布的创作规则和方法。在 Agent 中选择需要的
          Skills，每个 Agent 独立生效。
        </p>
      </div>
      <div className="hub-section-head">
        <h3>能力库</h3>
        <button onClick={openAgents}>去 Agent 装配</button>
      </div>
      {skills.map((s) => (
        <article className="hub-feature" key={s.id}>
          <div className="hub-section-head">
            <div>
              <h3>{s.name}</h3>
              <p>{s.description}</p>
            </div>
            <span className="model-badge">{s.kind}</span>
          </div>
          <p className="model-hint">
            {hub.agents.filter((a) => a.skillIds.includes(s.id)).length} 个
            Agent 已装配 ·{" "}
            {installed.find((v) => v.id === s.id)?.available
              ? "内置规则可用"
              : "内置规则待检查"}
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
