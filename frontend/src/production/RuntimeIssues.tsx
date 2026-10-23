import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { ErrorNotice } from "../errors/ErrorNotice";
import { t } from "../i18n";

type RuntimeIssue = {
  runner: string;
  projectId: string | null;
  stopped: boolean;
  error: string;
};

export function useRuntimeIssues(projectId: string) {
  const [issues, setIssues] = useState<RuntimeIssue[]>([]);
  const [error, setError] = useState<unknown>();
  useEffect(() => {
    if (!native) return;
    let active = true;
    let revision = 0;
    const refresh = async () => {
      const current = ++revision;
      try {
        const next = await bridge<RuntimeIssue[]>("generation_runtime_issues");
        if (active && current === revision) {
          setIssues(next);
          setError(undefined);
        }
      } catch (failure) {
        if (active && current === revision) setError(failure);
      }
    };
    const subscription = listen("generation-runtime-changed", refresh);
    void subscription.then(refresh).catch(setError);
    return () => {
      active = false;
      void subscription.then(
        (off) => off(),
        () => {},
      );
    };
  }, []);
  return {
    issues: issues.filter(
      (issue) => !issue.projectId || issue.projectId === projectId,
    ),
    error,
  };
}

export function RuntimeIssues({
  issues,
  error,
}: ReturnType<typeof useRuntimeIssues>) {
  if (!issues.length && !error) return null;
  return (
    <div className="task-runtime-issues">
      <ErrorNotice error={error} fallback="JOB_SYNC_FAILED" />
      {issues.map((issue) => (
        <ErrorNotice
          key={`${issue.runner}:${issue.projectId}:${issue.error}`}
          error={issue.error}
        >
          <span>
            {t(
              issue.stopped
                ? "后台处理已停止，请处理错误后重启应用。"
                : "本次启动未完成项目任务同步，请查看错误详情。",
            )}
          </span>
        </ErrorNotice>
      ))}
    </div>
  );
}
