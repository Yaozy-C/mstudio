import type { Project } from "../model";
import type { ProductionItem } from "./types";
export function ShotCardText({
  item,
  project,
}: {
  item: ProductionItem;
  project: Project;
}) {
  const node = project.nodes.find((n) => n.id === item.nodeId);
  const shot = node?.shot;
  const chapter = project.nodes
    .find((n) => n.id === shot?.planId)
    ?.plan?.script?.find((s) => s.id === shot?.scriptId);
  const action = node?.text || chapter?.action || "尚未填写画面描述";
  const dialogue = shot?.dialogue || chapter?.dialogue;
  return (
    <div className="shot-card-content">
      <div className="shot-card-meta">
        <span>镜头 {String(shot?.order ?? 1).padStart(2, "0")}</span>
        <strong className="shot-card-duration">
          {shot?.duration != null
            ? `${Number(shot.duration.toFixed(2))} 秒`
            : "时长待定"}
        </strong>
      </div>
      {chapter && (
        <div className="shot-card-chapter">
          所属章节 · {chapter.title || "未命名章节"}
        </div>
      )}
      <div className="shot-card-sections">
        {action
          .split(/(?=【[^】]+】)/u)
          .filter(Boolean)
          .map((part, i) => {
            const match = part.match(/^【([^】]+)】\s*([\s\S]*)$/u);
            return (
              <section key={i}>
                <h3>{match?.[1] || "画面与动作"}</h3>
                <p>{match?.[2] ?? part}</p>
              </section>
            );
          })}
        {dialogue && (
          <section>
            <h3>台词 / 旁白</h3>
            <p>{dialogue}</p>
          </section>
        )}
      </div>
    </div>
  );
}
