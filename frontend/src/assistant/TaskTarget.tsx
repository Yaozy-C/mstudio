import { X } from "@phosphor-icons/react";
import type { Project } from "../model";
export function TaskTarget({
  project,
  label,
  id,
  remove,
}: {
  label?: string;
  project: Project;
  id?: string | null;
  remove?: () => void;
}) {
  if (!id) return null;
  const node = project.nodes.find((n) => n.id === id);
  return (
    <div className="agent-task-target">
      <span>{remove ? "正在编辑：" : "关联对象："}</span>
      <strong title={[node?.title, label].filter(Boolean).join(" · ")}>
        {label || node?.title || "对象已移除，请重新选择"}
      </strong>
      {remove && (
        <button
          type="button"
          aria-label="取消本轮关联"
          title="取消本轮关联"
          onClick={remove}
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
}
