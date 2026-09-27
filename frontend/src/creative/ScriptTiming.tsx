import { t, useLanguage } from "../i18n";
import { useEffect, useState } from "react";
import type { Project } from "../model";
import type { ScriptParagraph } from "./types";
import { retimeScript, scriptDuration, timeLabel } from "./timing";
import "../styles/script-timing.css";
export function DurationInput({
  label,
  value,
  commit,
  min = 0.01,
}: {
  label: string;
  value: number;
  commit: (seconds: number) => void;
  min?: number;
}) {
  useLanguage();
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  return (
    <label className="script-duration">
      {label}
      <input
        type="number"
        aria-label={label}
        min={min}
        max={3600}
        step="0.01"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => {
          const n = Math.round(Number(draft) * 100) / 100;
          if (draft && Number.isFinite(n) && n >= min && n <= 3600) commit(n);
          else setDraft(String(value));
        }}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            setDraft(String(value));
            e.preventDefault();
          }
        }}
      />
      {t("秒")}
    </label>
  );
}
export function ScriptTiming({
  script,
  planId,
  change,
}: {
  script: ScriptParagraph[];
  planId: string;
  change: (f: (p: Project) => Project) => void;
}) {
  useLanguage();
  const total = scriptDuration(script);
  return (
    <section className="script-timing" aria-label={t("脚本时间")}>
      <div>
        <DurationInput
          label={t("脚本总时长")}
          value={total}
          min={script.length / 100}
          commit={(seconds) =>
            change((p) => ({
              ...p,
              nodes: p.nodes.map((n) =>
                n.id === planId
                  ? {
                      ...n,
                      plan: {
                        ...n.plan!,
                        script: retimeScript(n.plan?.script ?? [], seconds),
                      },
                    }
                  : n,
              ),
            }))
          }
        />
        <span>
          {timeLabel(total)} · {script.length} {t("段")}
        </span>
      </div>
      <p>
        {t(
          "修改总时长会按当前节奏分配到各段；修改单段会更新总时长。镜头和成片时间保持独立。",
        )}
      </p>
    </section>
  );
}
