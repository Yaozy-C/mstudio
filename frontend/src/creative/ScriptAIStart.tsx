import { t, useLanguage } from "../i18n";
import { requestCreativeTask } from "./aiTasks";
export function ScriptAIStart({
  planId,
  manual,
}: {
  planId?: string;
  manual: () => void;
}) {
  useLanguage();
  return (
    <section className="script-ai-start" aria-label={t("AI 脚本起草")}>
      <h2>{t("暂无脚本")}</h2>
      <div className="script-ai-actions">
        <button
          className="primary"
          onClick={() =>
            requestCreativeTask({
              text: "",
              agentId: "concept",
              targetNodeId: planId,
            })
          }
        >
          {t("让 Agent 起草脚本")}
        </button>
        <button onClick={manual}>{t("手动写作")}</button>
      </div>
    </section>
  );
}
