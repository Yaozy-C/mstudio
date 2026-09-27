import { t, useLanguage } from "../i18n";
import { Plus } from "@phosphor-icons/react";
export function NewTaskButton({
  active,
  disabled,
  onClick,
}: {
  active: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  useLanguage();
  return (
    <button
      type="button"
      className="agent-new-task"
      disabled={disabled}
      aria-label={t("作为新任务发送")}
      aria-pressed={active}
      title={
        active
          ? t("下一条作为新任务发送，点击改为继续任务")
          : t("开启新任务，保留聊天记录和项目约束")
      }
      onClick={onClick}
    >
      <Plus size={14} />
      {active ? t("取消新任务") : t("新任务")}
    </button>
  );
}
