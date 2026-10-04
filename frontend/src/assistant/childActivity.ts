import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { activityRows, type ActivityEvent } from "./activityRows";
export type ChildActivity = {
  projectId: string;
  parentTurn: string;
  turnId: string;
  childId: string;
  callId: string;
  agentId: string;
  state: "running" | "completed" | "cancelled" | "failed" | "interrupted";
  phase: { kind?: string; payload?: unknown };
  note: string;
  started: number;
  updated: number;
  events: ActivityEvent[];
};
export function belongsTo(child: ChildActivity, project: string, turn: string) {
  return child.projectId === project && child.parentTurn === turn;
}
export function childStatus(child: ChildActivity) {
  if (child.state === "completed") return "已完成";
  if (child.state === "cancelled") return "已停止";
  if (child.state === "failed") return "执行失败";
  if (child.state === "interrupted") return "执行已中断";
  if (child.phase.kind === "request/retry")
    return "模型请求暂时失败，正在重试当前请求…";
  if (child.phase.kind === "assistant/partial") return "正在回答…";
  if (child.phase.kind === "tool/call") {
    return activityRows([
      { seq: 0, created: 0, kind: "tool/call", payload: child.phase.payload },
    ])[0].title;
  }
  return "正在等待模型响应…";
}
export function elapsed(child: ChildActivity, now: number) {
  return Math.max(
    0,
    Math.floor(
      (child.state === "running" ? now : child.updated) - child.started,
    ),
  );
}
export function useChildActivity(projectId: string, turnId: string) {
  const [children, setChildren] = useState<ChildActivity[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    setChildren([]);
    setError("");
    if (!native) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    let revision = 0;
    const refresh = () => {
      clearTimeout(timer);
      const version = ++revision;
      timer = setTimeout(() => {
        void bridge<ChildActivity[]>("agent_child_activity", {
          projectId,
          turnId,
        })
          .then((items) => {
            if (active && version === revision) {
              setChildren(items);
              setError("");
            }
          })
          .catch((e) => {
            if (active && version === revision) setError(String(e));
          });
      }, 100);
    };
    const subscription = listen<ChildActivity>(
      "agent-child-progress",
      ({ payload }) => {
        if (active && belongsTo(payload, projectId, turnId)) refresh();
      },
    );
    void subscription.then(() => {
      if (active) refresh();
    });
    return () => {
      active = false;
      clearTimeout(timer);
      void subscription.then((off) => off());
    };
  }, [projectId, turnId]);
  return { children, error };
}
