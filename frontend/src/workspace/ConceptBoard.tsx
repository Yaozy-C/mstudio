import { Check, Lightbulb, NotePencil, Trash } from "@phosphor-icons/react";
import { ActionButton } from "../ui/ActionButton";
import { requestCreativeTask } from "../creative/aiTasks";
import { exploreConceptsTask } from "../creative/conceptTasks";
import { t, useLanguage } from "../i18n";
import type { ConceptStatus, Project } from "../model";
import "./concepts.css";

const statusLabel: Record<ConceptStatus, string> = {
  rough: "草稿",
  developed: "开发中",
  selected: "已选中",
  killed: "已淘汰",
};

export function ConceptBoard({
  project,
  change,
  write,
}: {
  project: Project;
  change: (f: (p: Project) => Project) => void;
  write: (id: string) => void;
}) {
  useLanguage();
  const cards = project.concepts ?? [];
  const status = (id: string, next: ConceptStatus) => {
    change((p) => ({
      ...p,
      concepts: (p.concepts ?? []).map((c) =>
        c.id === id ? { ...c, status: next, updatedAt: Date.now() } : c,
      ),
    }));
  };

  const remove = (id: string) => {
    change((p) => ({
      ...p,
      concepts: (p.concepts ?? []).filter((c) => c.id !== id),
    }));
  };

  const explore = () => requestCreativeTask(exploreConceptsTask());

  return (
    <section className="concept-workspace">
      <header className="concept-head">
        <div>
          <h1>{t("创意板")}</h1>
          <p>{t("先探索方向，再选中创意交给编剧。")}</p>
        </div>
        <div className="concept-head-actions">
          <span>{t("{count} 个方向", { count: cards.length })}</span>
          <ActionButton icon={Lightbulb} onClick={explore}>
            {t("探索方向")}
          </ActionButton>
        </div>
      </header>

      {cards.length === 0 ? (
        <div className="concept-empty">
          <Lightbulb size={34} />
          <h3>{t("还没有概念卡")}</h3>
          <p>
            {t(
              "告诉创意策划你的产品、受众和想法，先探索 8–12 个粗案，再开发 3 个方向。选中后可交给编剧写脚本。",
            )}
          </p>
          <button className="primary" onClick={explore}>
            <Lightbulb /> {t("探索方向")}
          </button>
          <p>{t("点击后会把任务带入 Agent 对话，检查后发送。")}</p>
        </div>
      ) : (
        <div className="concept-grid">
          {cards.map((c) => (
            <article key={c.id} className={`concept-card status-${c.status}`}>
              <div className="concept-card-top">
                <span className={`concept-status status-${c.status}`}>
                  {t(statusLabel[c.status])}
                </span>
                <div className="concept-card-tools">
                  <ActionButton
                    icon={Trash}
                    className="danger"
                    onClick={() => remove(c.id)}
                  >
                    {t("删除")}
                  </ActionButton>
                </div>
              </div>
              <h3>{c.title || t("未命名方向")}</h3>
              <p className="concept-one-line">{c.oneLine}</p>
              <dl>
                <Field label={t("真话")} value={c.truth ?? ""} strong />
                <Field label={t("品牌观")} value={c.brandIdea ?? ""} strong />
                <Field label={t("最不可能的人")} value={c.participant} />
                <Field label={t("他想要什么")} value={c.want} />
                <Field label={t("仪式")} value={c.ritual} />
                <Field label={t("产品证据")} value={c.evidence} />
                <Field label={t("最后一句")} value={c.reframe} strong />
                <Field label={t("风险")} value={c.risk} />
                {c.cost && <Field label={t("成本")} value={c.cost} />}
              </dl>
              <div className="concept-card-actions">
                {c.status === "selected" && (
                  <ActionButton icon={NotePencil} onClick={() => write(c.id)}>
                    {t("基于此创意写脚本")}
                  </ActionButton>
                )}
                <ActionButton
                  icon={Check}
                  aria-pressed={c.status === "selected"}
                  onClick={() =>
                    status(
                      c.id,
                      c.status === "selected" ? "developed" : "selected",
                    )
                  }
                >
                  {c.status === "selected" ? t("取消选中") : t("选中")}
                </ActionButton>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function Field({
  label,
  value,
  strong,
}: {
  label: string;
  value: string;
  strong?: boolean;
}) {
  if (!value) return null;
  return (
    <>
      <dt>{label}</dt>
      <dd className={strong ? "is-strong" : undefined}>{value}</dd>
    </>
  );
}
