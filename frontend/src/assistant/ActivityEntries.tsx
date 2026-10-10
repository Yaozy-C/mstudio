import { Fragment, type ReactNode } from "react";
import { CaretRight, ListChecks } from "@phosphor-icons/react";
import { StatusIcon } from "../ui/AsyncState";
import { t } from "../i18n";
import { redactDetails } from "../errors/catalog";
import type { ActivityEvent } from "./activityRows";
import { activityTimeline } from "./activityTimeline";

// Each event owns its disclosure; child tasks are rendered as sibling entries.
export function ActivityEntries({
  events,
  childName = (id) => id,
  afterTool,
}: {
  events: ActivityEvent[];
  childName?: (id: string) => string;
  afterTool?: (callId?: string) => ReactNode;
}) {
  return activityTimeline(events).map((entry) => {
    if (entry.kind === "progress")
      return (
        <p className="agent-progress-note" key={entry.id}>
          {entry.text}
        </p>
      );
    const row = entry.tool;
    const call = events.find((event) => event.seq === entry.id)?.payload as
      { name?: string; arguments?: unknown } | undefined;
    const result =
      row &&
      (events.find(
        (event) =>
          event.kind === "tool/result" &&
          (event.payload as { callId?: string })?.callId === row.callId,
      )?.payload as { result?: unknown } | undefined);
    return (
      <Fragment key={entry.id}>
        <details className={`agent-activity-item agent-activity-${entry.kind}`}>
          <summary className="agent-activity-row">
            <CaretRight
              className="agent-activity-chevron"
              size={14}
              aria-hidden="true"
            />
            {row ? (
              <>
                {row.detail === "执行中" ? (
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
          </summary>
          <div className="agent-activity-item-body">
            {row ? (
              <>
                <div className="agent-tool-name">{call?.name}</div>
                {call?.arguments !== undefined && (
                  <>
                    <div>{t("调用参数")}</div>
                    <pre>
                      {redactDetails(JSON.stringify(call.arguments, null, 2))}
                    </pre>
                  </>
                )}
                {result?.result !== undefined && (
                  <>
                    <div>{t("返回结果")}</div>
                    <pre>
                      {redactDetails(JSON.stringify(result.result, null, 2))}
                    </pre>
                  </>
                )}
              </>
            ) : (
              <p className="agent-progress-note">{entry.text}</p>
            )}
          </div>
        </details>
        {row && afterTool?.(row.callId)}
      </Fragment>
    );
  });
}
