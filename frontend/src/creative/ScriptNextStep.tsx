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
        <h2>{t("设计整片镜头")}</h2>
        {count > 0 && <p>{t("{v0} 个镜头", { v0: count })}</p>}
      </div>
      <button className="primary" onClick={next}>
        {count ? t("调整整片镜头") : t("根据完整脚本设计镜头")}
      </button>
    </section>
  );
}
