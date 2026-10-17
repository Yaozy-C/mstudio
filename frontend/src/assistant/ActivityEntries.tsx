import { Fragment, type ReactNode } from "react";
import { CaretRight, ListChecks } from "@phosphor-icons/react";
import { StatusIcon } from "../ui/AsyncState";
import { t } from "../i18n";
import { LazyDetails } from "./LazyDetails";
import { ActivityDetails } from "./ActivityDetails";
import type { ActivityEvent } from "./activityRows";
import { activityTimeline } from "./activityTimeline";

// Each event owns its disclosure; child tasks are rendered as sibling entries.
export function ActivityEntries({
  events,
  childName = (id) => id,
  afterTool,
  toolState,
}: {
  events: ActivityEvent[];
  childName?: (id: string) => string;
  afterTool?: (callId?: string) => ReactNode;
  toolState?: (
    callId?: string,
  ) => { detail: string; error: boolean } | undefined;
}) {
  return activityTimeline(events).map((entry) => {
    if (entry.kind === "progress")
      return (
        <p className="agent-progress-note" key={entry.id}>
          {entry.text}
        </p>
      );
    const saved = entry.tool;
    const live =
      saved && ["执行中", "子任务执行中"].includes(saved.detail)
        ? toolState?.(saved.callId)
        : undefined;
    const row = saved && live ? { ...saved, ...live } : saved;
    return (
      <Fragment key={entry.id}>
        <LazyDetails
          id={`activity:${entry.id}`}
          className={`agent-activity-item agent-activity-${entry.kind}`}
          summaryClassName="agent-activity-row"
          summary={
            <>
              <CaretRight
                className="agent-activity-chevron"
                size={14}
                aria-hidden="true"
              />
              {row ? (
                <>
                  {["执行中", "子任务执行中"].includes(row.detail) ? (
                    <StatusIcon kind="loading" size={16} />
                  ) : (
                    <ListChecks size={16} aria-hidden="true" />
                  )}
                  <span>
                    {row.agentId
                      ? t("委派给 {v0}", { v0: childName(row.agentId) })
                      : row.title
                          .split("、")
                          .map((part) => t(part))
                          .join(" · ")}
                  </span>
                  <small className={row.error ? "error" : ""}>
                    {t(row.detail)}
                  </small>
                </>
              ) : (
                <>
                  <span>
                    {entry.kind === "thought" ? t("思考") : t("已加载规则")}
                  </span>
                  <span className="agent-activity-preview">
                    {entry.text.split("\n").find((line) => line.trim())}
                  </span>
                </>
              )}
            </>
          }
        >
          {() => (
            <div className="agent-activity-item-body">
              {row ? (
                <ActivityDetails
                  events={events}
                  id={entry.id}
                  callId={row.callId}
                />
              ) : (
                <p className="agent-progress-note">{entry.text}</p>
              )}
            </div>
          )}
        </LazyDetails>
        {row && afterTool?.(row.callId)}
      </Fragment>
    );
  });
}
