import { t, useLanguage } from "../i18n";
import "../styles/creative-next-step.css";
export function ScriptNextStep({
  hasScript,
  count,
  next,
}: {
  hasScript: boolean;
  count: number;
  next: () => void;
}) {
  useLanguage();
  if (!hasScript) return null;
  return (
    <section className="creative-next-step" aria-label={t("脚本下一步")}>
      <div>
        <small>{t("下一步")}</small>
        <h2>{t("设计整片镜头")}</h2>
        <p>
          {count
            ? t("已有 {v0} 个镜头，继续检查镜头设计、制作分镜图。", {
                v0: count,
              })
            : t("先设计整片的观看顺序、节奏与切点，再确定镜头和关键画格。")}
        </p>
      </div>
      <button className="primary" onClick={next}>
        {count ? t("调整整片镜头") : t("根据完整脚本设计镜头")}
      </button>
    </section>
  );
}
