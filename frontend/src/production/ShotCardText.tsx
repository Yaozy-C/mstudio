import { t, useLanguage } from "../i18n";
import type { Project } from "../model";
import type { ProductionItem } from "./types";
export function ShotCardText({
  item,
  project,
}: {
  item: ProductionItem;
  project: Project;
}) {
  useLanguage();
  const node = project.nodes.find((n) => n.id === item.nodeId);
  const shot = node?.shot;
  const chapter = project.nodes
    .find((n) => n.id === shot?.screenplayId)
    ?.screenplay?.script?.find((s) => s.id === shot?.scriptId);
  const action = node?.text || chapter?.action || t("尚未填写画面描述");
  const dialogue = shot?.dialogue || chapter?.dialogue;
  return (
    <div className="shot-card-content">
      <div className="shot-card-meta">
        <span>
          {t("镜头")} {String(shot?.order ?? 1).padStart(2, "0")}
        </span>
        <strong className="shot-card-duration">
          {shot?.duration != null
            ? t("{v0} 秒", { v0: Number(shot.duration.toFixed(2)) })
            : t("时长待定")}
        </strong>
      </div>
      {chapter && (
        <div className="shot-card-chapter">
          {t("所属章节 ·")} {chapter.title || t("未命名章节")}
        </div>
      )}
      <div className="shot-card-sections">
        {!!node?.references?.length && (
          <section>
            <h3>{t("参考素材")}</h3>
            <p>
              {node.references
                .map(
                  (r) =>
                    project.assets.find((a) => a.id === r.assetId)?.name ??
                    r.assetId,
                )
                .join("；")}
            </p>
          </section>
        )}
        {action
          .split(/(?=【[^】]+】)/u)
          .filter(Boolean)
          .map((part, i) => {
            const match = part.match(/^【([^】]+)】\s*([\s\S]*)$/u);
            return (
              <section key={i}>
                <h3>{match?.[1] || t("画面与动作")}</h3>
                <p>{match?.[2] ?? part}</p>
              </section>
            );
          })}
        {dialogue && (
          <section>
            <h3>{t("台词 / 旁白")}</h3>
            <p>{dialogue}</p>
          </section>
        )}
      </div>
    </div>
  );
}
