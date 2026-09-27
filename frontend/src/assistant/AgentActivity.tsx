import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { activityRows, type ActivityEvent } from "./activityRows";
type Event = ActivityEvent;
type Progress = {
  projectId: string;
  turnId: string;
  kind: string;
  payload?: { step?: number; action?: string };
};
const actions: Record<string, string> = {
  inspect: "读取项目",
  edit: "修改项目",
  skills: "查找能力",
  read_skill: "读取 Skill",
  history: "翻查对话",
};
export function AgentActivity({
  projectId,
  turnId,
  running,
}: {
  projectId: string;
  turnId: string;
  running: boolean;
}) {
  useLanguage();
  const [events, setEvents] = useState<Event[]>([]);
  const [error, setError] = useState("");
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!native || !running) return;
    let active = true;
    const subscription = listen<Progress>(
      "agent-progress",
      ({ payload: e }) => {
        if (!active || e.projectId !== projectId || e.turnId !== turnId) return;
        setStatus(
          e.kind === "turn/end"
            ? ""
            : e.kind === "step/start"
              ? t("第 {v0} 步 · 正在思考", { v0: e.payload?.step })
              : e.kind === "tool/start"
                ? actions[e.payload?.action || ""] || t("执行操作")
                : e.kind === "memory/result"
                  ? t("整理项目记忆")
                  : e.kind === "request/retry"
                    ? t("模型请求暂时失败，正在重试当前请求…")
                    : e.kind === "request/start"
                      ? t("正在等待模型响应…")
                      : e.kind === "assistant/partial"
                        ? t("正在回答…")
                        : e.kind === "tool/call"
                          ? t("正在执行工具…")
                          : t("核查结果"),
        );
        if (e.kind !== "assistant/partial") setRevision((v) => v + 1);
      },
    );
    return () => {
      active = false;
      void subscription.then((off) => off());
    };
  }, [projectId, turnId, running]);
  useEffect(() => {
    if (!native) return;
    let active = true;
    const timer = setTimeout(() => {
      void bridge<Event[]>("agent_turn_events", {
        projectId,
        turnId,
        before: null,
      })
        .then((v) => {
          if (active) {
            setEvents(v);
            setError("");
          }
        })
        .catch((e) => {
          if (active) setError(String(e));
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [projectId, turnId, open, revision, running]);
  const rows = activityRows(events);
  const scripts = new Map<string, { id: string; title: string }>();
  for (const event of [...events].sort((a, b) => a.seq - b.seq)) {
    const result = (
      event.payload as {
        result?: {
          applied?: boolean;
          scripts?: { id: string; title: string }[];
        };
      }
    )?.result;
    if (event.kind === "tool/result" && result?.applied)
      for (const script of result.scripts ?? []) scripts.set(script.id, script);
  }
  return (
    <>
      {running && (
        <div className="agent-run-status" role="status">
          {status || t("正在处理任务…")}
        </div>
      )}
      {[...scripts.values()].map((s) => (
        <div className="agent-script-receipt" key={s.id}>
          <span>
            {t("已更新脚本 ·")} {s.title}
          </span>
          <button
            className="text-button"
            onClick={() =>
              window.dispatchEvent(
                new CustomEvent("studio-show-script", { detail: s.id }),
              )
            }
          >
            {t("查看脚本")}
          </button>
        </div>
      ))}
      <details
        className="agent-activity"
        onToggle={(e) => setOpen(e.currentTarget.open)}
      >
        <summary>
          {rows.length
            ? t("已记录 {v0} 项操作", { v0: rows.length })
            : running
              ? t("正在执行")
              : t("执行记录")}
        </summary>
        {error && <ErrorNotice error={error} />}
        {events
          .filter((e) => e.kind === "skill/loaded")
          .map((e) => {
            const rule = e.payload as { skill: string; path: string };
            return (
              <div className="agent-activity-row" key={`skill-${e.seq}`}>
                <span>{t("已加载规则")}</span>
                <small>
                  {rule.skill} · {rule.path}
                </small>
              </div>
            );
          })}
        {rows.map((r) => (
          <div className="agent-activity-row" key={r.id}>
            <span>
              {r.title
                .split("、")
                .map((part) => t(part))
                .join(" · ")}
            </span>
            <small className={r.error ? "error" : ""}>{t(r.detail)}</small>
          </div>
        ))}
        {!events.length && !error && <p>{t("尚无执行记录。")}</p>}
      </details>
    </>
  );
}
