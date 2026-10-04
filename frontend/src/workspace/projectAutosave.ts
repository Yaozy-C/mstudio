import { mergeValue } from "../domain/merge";
import { normalizeError } from "../errors/catalog";
import type { Project } from "../model";

export const AUTOSAVE_DELAY = 1500;
const SAVING_INDICATOR_DELAY = 300;
export const SAVED_LABEL = "已保存到本机";
export const SAVE_DESCRIPTION =
  "自动保存脚本、分镜关联、素材记录、时间线和画布布局；停止操作 1.5 秒后保存到本机。";

export function createProjectAutosave(
  initial: Project,
  write: (project: Project, base: Project) => Promise<Project>,
  reconcile: (project: Project, remote: boolean) => void = () => {},
) {
  let latest = initial;
  let stored = initial;
  let status = SAVED_LABEL;
  let conflicted = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let indicator: ReturnType<typeof setTimeout> | undefined;
  let saving: Promise<void> | undefined;
  const listeners = new Set<() => void>();
  const isDirty = () => latest !== stored;
  function report(value: string) {
    if (status === value) return;
    status = value;
    listeners.forEach((listener) => listener());
  }
  function clearTimer() {
    clearTimeout(timer);
    timer = undefined;
  }
  function writeLatest(): Promise<void> {
    if (saving) return saving;
    if (!isDirty()) return Promise.resolve();
    const snapshot = latest;
    const base = stored;
    indicator = setTimeout(() => report("保存中…"), SAVING_INDICATOR_DELAY);
    saving = Promise.resolve()
      .then(() => write(snapshot, base))
      .then((committed) => {
        if ((committed.storageVersion ?? 0) < (stored.storageVersion ?? 0))
          committed = stored;
        latest = mergeValue(snapshot, latest, committed) as Project;
        stored = committed;
        conflicted = false;
        reconcile(latest, false);
        if (latest === stored) report(SAVED_LABEL);
      })
      .catch((error) => {
        conflicted = normalizeError(error).code === "PROJECT_CONFLICT";
        report(`保存失败：${normalizeError(error, "SAVE_FAILED").message}`);
        throw error;
      })
      .finally(() => {
        clearTimeout(indicator);
        saving = undefined;
      });
    return saving;
  }
  async function saveWhenIdle() {
    if (saving) await saving.catch(() => {});
    // An edit during an in-flight write starts a new debounce window.
    if (timer === undefined) await writeLatest();
  }
  return {
    isDirty,
    isConflicted: () => conflicted,
    resolve(remote: Project, choice: "local" | "current") {
      if (saving) throw new Error("请等待当前保存完成");
      latest =
        choice === "current"
          ? remote
          : (mergeValue(stored, latest, remote, "", "local") as Project);
      stored = remote;
      conflicted = false;
      reconcile(latest, false);
      report(isDirty() ? "待保存" : SAVED_LABEL);
    },
    accept(document: Project) {
      if ((document.storageVersion ?? 0) < (stored.storageVersion ?? 0)) return;
      try {
        const merged = mergeValue(stored, latest, document) as Project;
        stored = document;
        latest = merged;
        reconcile(latest, true);
        if (!isDirty()) report(SAVED_LABEL);
      } catch (error) {
        conflicted = normalizeError(error).code === "PROJECT_CONFLICT";
        report(`保存失败：${normalizeError(error, "SAVE_FAILED").message}`);
        throw error;
      }
    },
    getStatus: () => status,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    update(project: Project) {
      if (latest === project) return;
      latest = project;
      clearTimer();
      clearTimeout(indicator);
      report("待保存");
      timer = setTimeout(() => {
        timer = undefined;
        void saveWhenIdle().catch(() => {});
      }, AUTOSAVE_DELAY);
    },
    async flush() {
      clearTimer();
      while (isDirty()) {
        await writeLatest();
        clearTimer();
      }
    },
  };
}
