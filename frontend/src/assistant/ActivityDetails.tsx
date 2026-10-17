import { t } from "../i18n";
import { redactDetails } from "../errors/catalog";
import type { ActivityEvent } from "./activityRows";

export function ActivityDetails({
  events,
  id,
  callId,
}: {
  events: ActivityEvent[];
  id: number;
  callId?: string;
}) {
  const call = events.find((event) => event.seq === id)?.payload as
    { name?: string; arguments?: unknown } | undefined;
  const result = events.find(
    (event) =>
      event.kind === "tool/result" &&
      (event.payload as { callId?: string })?.callId === callId,
  )?.payload as { result?: unknown } | undefined;
  return (
    <>
      <div className="agent-tool-name">{call?.name}</div>
      {call?.arguments !== undefined && (
        <>
          <div>{t("调用参数")}</div>
          <pre>{redactDetails(JSON.stringify(call.arguments, null, 2))}</pre>
        </>
      )}
      {result?.result !== undefined && (
        <>
          <div>{t("返回结果")}</div>
          <pre>{redactDetails(JSON.stringify(result.result, null, 2))}</pre>
        </>
      )}
    </>
  );
}
