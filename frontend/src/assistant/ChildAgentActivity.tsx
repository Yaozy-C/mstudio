import { useEffect, useState } from "react";
import { t } from "../i18n";
import { activityRows } from "./activityRows";
import { ActivityEntries } from "./ActivityEntries";
import { childStatus, elapsed, type ChildActivity } from "./childActivity";
import "./child-activity.css";
export function ChildAgentActivity({
  child,
  name,
  showTools = false,
}: {
  child: ChildActivity;
  name: string;
  showTools?: boolean;
}) {
  const [now, setNow] = useState(Date.now() / 1000);
  useEffect(() => {
    if (child.state !== "running") return;
    setNow(Date.now() / 1000);
    const timer = setInterval(() => setNow(Date.now() / 1000), 1000);
    return () => clearInterval(timer);
  }, [child.state, child.turnId]);
  const seconds = elapsed(child, now);
  const rows = activityRows(child.events);
  return (
    <div className="agent-child-activity" data-state={child.state}>
      <div className="agent-child-heading">
        <strong>{name}</strong>
        <span>
          {t("已用时 {v0}分 {v1}秒", {
            v0: Math.floor(seconds / 60),
            v1: seconds % 60,
          })}
        </span>
      </div>
      <div className="agent-child-phase" role="status">
        {t(childStatus(child))}
      </div>
      {child.state === "running" && child.note && (
        <p className="agent-child-note">{child.note}</p>
      )}
      {showTools && (
        <details>
          <summary>{t("子任务最近 {v0} 项操作", { v0: rows.length })}</summary>
          <ActivityEntries events={child.events} />
        </details>
      )}
    </div>
  );
}
