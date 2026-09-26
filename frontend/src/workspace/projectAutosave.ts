import { normalizeError } from "../errors/catalog";
import type { Project } from "../model";

export const AUTOSAVE_DELAY = 1500;
const SAVING_INDICATOR_DELAY = 300;
export const SAVED_LABEL = "已保存到本机";
export const SAVE_DESCRIPTION =
  "自动保存脚本、分镜关联、素材记录、时间线和画布布局；停止操作 1.5 秒后保存到本机。";

export function createProjectAutosave(
  initial: Project,
  write: (project: Project) => Promise<void>,
) {
  let latest = initial;
  let stored = initial;
  let status = SAVED_LABEL;
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
    indicator = setTimeout(() => report("保存中…"), SAVING_INDICATOR_DELAY);
    saving = Promise.resolve()
      .then(() => write(snapshot))
      .then(() => {
        stored = snapshot;
        if (latest === snapshot) report(SAVED_LABEL);
      })
      .catch((error) => {
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
