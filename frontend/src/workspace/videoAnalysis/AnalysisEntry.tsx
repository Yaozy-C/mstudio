import { ArrowRight } from "@phosphor-icons/react";
import { native } from "../../bridge";
import { t } from "../../i18n";
import { browseWorks } from "../browseWorks";
import type { AnalysisController } from "./controller";
import { SourcePicker } from "./SourcePicker";

const references = ["mora-mora", "vero-factory", "solace-vlad"].flatMap((id) =>
  browseWorks.filter((work) => work.id === id),
);
export function AnalysisEntry({
  controller,
  busy,
  onBrowse,
}: {
  controller: AnalysisController;
  busy: boolean;
  onBrowse: () => void;
}) {
  return (
    <div className="analysis-entry">
      <section
        className="analysis-welcome"
        aria-labelledby="analysis-welcome-title"
      >
        <h2 id="analysis-welcome-title">{t("好作品，值得细看。")}</h2>
        <p>{t("从一部好作品，读懂创意和声画的关系。")}</p>
        <SourcePicker controller={controller} busy={busy} />
      </section>
      <section
        className="analysis-references"
        aria-labelledby="analysis-references-title"
      >
        <header>
          <h2 id="analysis-references-title">{t("从作品开始")}</h2>
          <button className="analysis-text-action" onClick={onBrowse}>
            {t("查看全部")}
            <ArrowRight size={18} aria-hidden="true" />
          </button>
        </header>
        <div className="analysis-reference-grid">
          {references.map((work) => (
            <button
              className="analysis-reference"
              key={work.id}
              disabled={busy}
              onClick={() => controller.selectWork(work)}
              aria-label={t("拆解作品 {title}", { title: work.title })}
            >
              <img src={work.poster} alt="" loading="lazy" />
              <h3>{work.title}</h3>
              <p>
                {work.credit} · {t("AI 概念广告")}
              </p>
            </button>
          ))}
        </div>
      </section>
      {!native && (
        <p className="analysis-preview-note">
          {t("可在此预览本地视频，AI 拆解请使用桌面应用。")}
        </p>
      )}
    </div>
  );
}
