import { StatusIcon } from "../ui/AsyncState";
import { t } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { activityRows, type ActivityEvent } from "./activityRows";
import { ActivityEntries } from "./ActivityEntries";
import { ChildAgentActivity } from "./ChildAgentActivity";
import type { ChildActivity } from "./childActivity";
import type { AgentProfile } from "../agents/catalog";

export function AgentActivityFeed({
  events,
  children = [],
  agents = [],
  seconds,
  running,
  status,
  error,
}: {
  events: ActivityEvent[];
  children?: ChildActivity[];
  agents?: Pick<AgentProfile, "id" | "name">[];
  seconds: number;
  running: boolean;
  status?: string;
  error?: string;
}) {
  const rows = activityRows(events);
  const childName = (id: string) =>
    agents.find((agent) => agent.id === id)?.name || id;
  const renderChild = (child: ChildActivity) => (
    <ChildAgentActivity
      key={child.turnId}
      child={child}
      name={childName(child.agentId)}
      showTools
    />
  );
  return (
    <div className="agent-activity agent-activity-timeline">
      {error && <ErrorNotice error={error} />}
      <div className="agent-activity-heading">
        {t("已处理 {v0}分 {v1}秒", {
          v0: Math.floor(seconds / 60),
          v1: seconds % 60,
        })}
      </div>
      <div className="agent-activity-content">
        <ActivityEntries
          events={events}
          childName={childName}
          afterTool={(callId) =>
            children.filter((child) => child.callId === callId).map(renderChild)
          }
        />
        {children
          .filter((child) => !rows.some((row) => row.callId === child.callId))
          .map(renderChild)}
      </div>
      {running && (
        <div className="agent-activity-status" role="status">
          <StatusIcon kind="loading" size={14} />
          {status || t("正在思考…")}
        </div>
      )}
    </div>
  );
}
