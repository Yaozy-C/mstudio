import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { type ActivityEvent } from "./activityRows";
import type { AgentProfile } from "../agents/catalog";
import { useChildActivity } from "./childActivity";
import { withLiveThinking } from "./activityTimeline";
import { AgentActivityFeed } from "./AgentActivityFeed";
type Event = ActivityEvent;
type Progress = {
  projectId: string;
  turnId: string;
  kind: string;
  payload?: { id?: string; step?: number; action?: string; text?: string };
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
  agents,
  startedAt,
}: {
  projectId: string;
  turnId: string;
  running: boolean;
  startedAt?: number;
  agents: Pick<AgentProfile, "id" | "name">[];
}) {
  useLanguage();
  const [events, setEvents] = useState<Event[]>([]);
  const [error, setError] = useState("");
  const [liveThinking, setLiveThinking] = useState<
    { id: string; text: string }[]
  >([]);
  const [started] = useState(Date.now());
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [running]);
  const [status, setStatus] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!native || !running) return;
    let active = true;
    const subscription = listen<Progress>(
      "agent-progress",
      ({ payload: e }) => {
        if (!active || e.projectId !== projectId || e.turnId !== turnId) return;
        if (e.kind === "assistant/thinking") {
          const id = e.payload?.id;
          if (id)
            setLiveThinking((current) => {
              const block = { id, text: e.payload?.text ?? "" };
              return current.some((item) => item.id === id)
                ? current.map((item) => (item.id === id ? block : item))
                : [...current, block];
            });
          setStatus(t("正在思考…"));
          return;
        }
        setStatus(
          e.kind === "assistant/progress"
            ? t("正在处理任务…")
            : e.kind === "turn/end"
              ? ""
              : e.kind === "step/start"
                ? t("第 {v0} 步 · 正在思考", { v0: e.payload?.step })
                : e.kind === "tool/start"
                  ? actions[e.payload?.action || ""] || t("执行操作")
                  : e.kind === "request/retry"
                    ? t("模型请求暂时失败，正在重试当前请求…")
                    : e.kind === "request/start"
                      ? t("正在等待模型响应…")
                      : e.kind === "assistant/partial"
                        ? t("正在回答…")
                        : e.kind === "tool/call"
                          ? t("正在执行工具…")
                          : t("正在处理任务…"),
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
      const read = async () => {
        const all: Event[] = [];
        let before: number | null = null;
        while (active) {
          const page: Event[] = await bridge<Event[]>("agent_turn_events", {
            projectId,
            turnId,
            before,
          });
          all.push(...page);
          if (page.length < 40) break;
          const next = Math.min(...page.map((event) => event.seq));
          if (before !== null && next >= before) break;
          before = next;
        }
        return all;
      };
      void read()
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
  }, [projectId, turnId, revision, running]);
  const { children, error: childError } = useChildActivity(projectId, turnId);
  const times = events.map((event) => event.created).filter((time) => time > 0);
  const begin =
    startedAt ?? (times.length ? Math.min(...times) * 1000 : started);
  const end = running ? now : times.length ? Math.max(...times) * 1000 : now;
  const seconds = Math.max(0, Math.floor((end - begin) / 1000));
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
      {childError && <ErrorNotice error={childError} />}
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
      <AgentActivityFeed
        events={withLiveThinking(events, liveThinking)}
        children={children}
        agents={agents}
        seconds={seconds}
        running={running}
        status={status}
        error={error}
      />
    </>
  );
}
