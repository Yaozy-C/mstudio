import { useState } from "react";
import type { Project } from "../model";
import type { ChangeProject, ProductionTask } from "./types";
import { saveTask } from "./document";
import {
  editTaskPrompt,
  canEditOriginal,
  regenerationDraft,
} from "./taskEditing";

export function useTaskPanel(
  runs: ProductionTask[],
  change: ChangeProject,
  get: () => Project,
  open: () => void,
  setComposerMode: (mode: "agent" | "image" | "video") => void,
) {
  const [taskPanelOpen, setTaskPanelOpen] = useState(false);
  const [activeTaskKey, setActiveTaskKey] = useState<string | null>(null);
  const [referencedTaskKey, setReferencedTaskKey] = useState<string | null>(
    null,
  );
  const [configuration, configure] = useState<ProductionTask | null>(null);
  return {
    taskPanelOpen,
    setTaskPanelOpen,
    activeTaskKey,
    clearActiveTask: () => setActiveTaskKey(null),
    showTask: (task: ProductionTask) => {
      setActiveTaskKey(task.key);
      setTaskPanelOpen(true);
    },
    referencedTask: runs.find((t) => t.key === referencedTaskKey),
    clearTaskReference: (key?: string) =>
      setReferencedTaskKey((current) =>
        !key || current === key ? null : current,
      ),
    referenceTask: (task: ProductionTask) => {
      setTaskPanelOpen(false);
      setReferencedTaskKey(task.key);
      setComposerMode("agent");
      open();
      window.dispatchEvent(
        new CustomEvent("studio-creative-request", {
          detail: {
            text: "",
            agentId: task.kind === "image" ? "image" : "production",
          },
        }),
      );
    },
    editPrompt: (key: string, text: string) =>
      change((p) => editTaskPrompt(p, key, text), false),
    hideTask: (key: string, hidden: boolean) =>
      change((p) => {
        const current = p.production?.drafts?.[key];
        return current
          ? saveTask(p, { ...current, hiddenFromList: hidden })
          : p;
      }, false),
    retry: (task: ProductionTask) =>
      change((p) => {
        const current = p.production?.drafts?.[task.key];
        if (
          !current ||
          !["FAILED", "CANCELLED", "COMPLETED"].includes(current.status ?? "")
        )
          return p;
        const retry = regenerationDraft(current);
        return saveTask(p, {
          ...retry,
          status: retry.modelId ? "READY" : "AWAITING_CONFIRMATION",
        });
      }, false),
    configuration,
    configure: (task: ProductionTask | null) => {
      if (!task) return configure(null);
      const current = get().production?.drafts?.[task.key] ?? task;
      configure(
        canEditOriginal(current) ? current : regenerationDraft(current),
      );
    },
  };
}
