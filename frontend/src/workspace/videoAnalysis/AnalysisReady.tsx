import {
  ArrowUpRight,
  ClockCounterClockwise,
  FilmStrip,
  Lightbulb,
} from "@phosphor-icons/react";
import { t } from "../../i18n";
export function AnalysisReady({ onHistory }: { onHistory: () => void }) {
  return (
    <section className="analysis-ready" aria-label={t("拆解结果")}>
      <header>
        <span className="analysis-ready-status">
          <span aria-hidden="true" />
          {t("准备就绪")}
        </span>
        <h2>{t("理解每一个创作选择。")}</h2>
        <p>{t("从声画细节到创意结构，把值得借鉴的部分留下来。")}</p>
      </header>
      <div className="analysis-ready-features">
        <div>
          <span className="analysis-ready-icon">
            <FilmStrip size={19} aria-hidden="true" />
          </span>
          <div>
            <h3>{t("逐段声画")}</h3>
            <p>{t("沿时间轴拆解画面、台词、文字与声音。")}</p>
          </div>
        </div>
        <div>
          <span className="analysis-ready-icon">
            <Lightbulb size={19} aria-hidden="true" />
          </span>
          <div>
            <h3>{t("创作方法")}</h3>
            <p>{t("理解创意内核、镜头选择与音乐节奏。")}</p>
          </div>
        </div>
      </div>
      <footer>
        <ClockCounterClockwise size={16} aria-hidden="true" />
        <p>{t("完成后自动保存，可随时回看。")}</p>
        <button onClick={onHistory} aria-label={t("查看拆解记录")}>
          <ArrowUpRight size={17} aria-hidden="true" />
        </button>
      </footer>
    </section>
  );
}
