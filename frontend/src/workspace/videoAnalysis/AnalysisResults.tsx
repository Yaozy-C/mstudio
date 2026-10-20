import { useState } from "react";
import { ArrowUpRight } from "@phosphor-icons/react";
import { AnalysisCreateAction } from "./AnalysisCreateAction";
import { AnalysisReady } from "./AnalysisReady";
import { t } from "../../i18n";
import { timecode, type VideoAnalysisResult } from "./types";

const craftLabels = {
  idea: "创意内核",
  visuals: "画面与美术",
  camera: "镜头与剪辑",
  language: "话语与文字",
  sound: "音乐与声音",
} as const;
export function AnalysisResults({
  result,
  seek,
  currentTime,
  onHistory = () => {},
  onCreate,
  continuing,
}: {
  result?: VideoAnalysisResult;
  seek: (time: number) => void;
  currentTime: number;
  onHistory?: () => void;
  onCreate?: () => Promise<void>;
  continuing?: boolean;
}) {
  const [tab, setTab] = useState<"segments" | "craft">("segments");
  if (!result) return <AnalysisReady onHistory={onHistory} />;
  return (
    <section className="analysis-results" aria-label={t("拆解结果")}>
      <header>
        <h2>{t("拆解结果")}</h2>
        <p>{result.summary}</p>
        <AnalysisCreateAction onCreate={onCreate} continuing={continuing} />
      </header>
      <div
        className="analysis-tabs"
        role="tablist"
        aria-label={t("拆解视图")}
        onKeyDown={(event) => {
          const index = ["ArrowLeft", "Home"].includes(event.key)
            ? 0
            : ["ArrowRight", "End"].includes(event.key)
              ? 1
              : -1;
          if (index < 0) return;
          event.preventDefault();
          setTab(index === 0 ? "segments" : "craft");
          event.currentTarget
            .querySelectorAll<HTMLButtonElement>("button")
            [index]?.focus();
        }}
      >
        <button
          id="analysis-segments-tab"
          role="tab"
          tabIndex={tab === "segments" ? 0 : -1}
          aria-selected={tab === "segments"}
          aria-controls="analysis-results-panel"
          onClick={() => setTab("segments")}
        >
          {t("逐段声画")}
        </button>
        <button
          id="analysis-craft-tab"
          role="tab"
          tabIndex={tab === "craft" ? 0 : -1}
          aria-selected={tab === "craft"}
          aria-controls="analysis-results-panel"
          onClick={() => setTab("craft")}
        >
          {t("创作方法")}
        </button>
      </div>
      <div
        id="analysis-results-panel"
        role="tabpanel"
        aria-labelledby={`analysis-${tab}-tab`}
      >
        {tab === "segments" ? (
          <div className="analysis-segments">
            {result.segments.map((segment, index) => (
              <article
                key={index}
                className="analysis-segment"
                data-current={
                  currentTime >= segment.start && currentTime < segment.end
                }
              >
                <button
                  className="analysis-time"
                  onClick={() => seek(segment.start)}
                  aria-label={t("跳转到 {time}", {
                    time: timecode(segment.start),
                  })}
                >
                  {timecode(segment.start)} — {timecode(segment.end)}
                  <ArrowUpRight size={14} aria-hidden="true" />
                </button>
                <h3>{segment.title}</h3>
                <dl>
                  {(
                    [
                      ["画面", segment.visual],
                      ["台词", segment.speech],
                      ["画面文字", segment.onScreenText],
                      ["声音", segment.sound],
                      ["表达作用", segment.purpose],
                    ] as const
                  )
                    .filter(([, value]) => value.trim())
                    .map(([label, value]) => (
                      <div key={label}>
                        <dt>{t(label)}</dt>
                        <dd>{value}</dd>
                      </div>
                    ))}
                </dl>
              </article>
            ))}
          </div>
        ) : (
          <div className="analysis-craft">
            {Object.entries(craftLabels).map(([key, label]) => (
              <article key={key}>
                <h3>{t(label)}</h3>
                <p>{result.craft[key as keyof typeof craftLabels]}</p>
              </article>
            ))}
          </div>
        )}
      </div>
      <p className="analysis-access">{result.access}</p>
    </section>
  );
}
