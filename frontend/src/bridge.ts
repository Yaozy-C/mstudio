import { commandError } from "./errors/commands";
import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import type { Project, ProjectEntry } from "./model";
export const native = isTauri();
export const mediaUrl = (path: string) =>
  !path
    ? ""
    : path.startsWith("blob:") || path.startsWith("data:")
      ? path
      : native
        ? convertFileSrc(path)
        : path;
export async function bridge<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (native) {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      throw new Error(commandError(command, error));
    }
  }
  if (command === "list_projects")
    return JSON.parse(localStorage.getItem("mstudio-preview") || "[]") as T;
  if (command === "save_project" || command === "create_project") {
    const p = args!.document as Project;
    const entries = await bridge<ProjectEntry[]>("list_projects");
    const exists = entries.some((e) => e.id === p.id);
    if (command === "save_project" && !exists) throw new Error("项目已删除");
    if (command === "create_project" && exists) throw new Error("项目已存在");
    const next = [
      { id: p.id, name: p.name, updated: Date.now() / 1000, document: p },
      ...entries.filter((e) => e.id !== p.id),
    ];
    localStorage.setItem("mstudio-preview", JSON.stringify(next));
    return undefined as T;
  }
  if (command === "delete_project") {
    const entries = await bridge<ProjectEntry[]>("list_projects");
    localStorage.setItem(
      "mstudio-preview",
      JSON.stringify(entries.filter((e) => e.id !== args!.id)),
    );
    return undefined as T;
  }
  if (command === "media_model_catalog") return [] as T;
  if (command === "list_global_assets") return [] as T;
  if (command === "get_settings")
    return {
      falConfigured: false,
    } as T;
  throw new Error(
    "此功能需要在 Mstudio 桌面应用中使用。浏览器仅提供界面预览。",
  );
}
let pending: Promise<unknown> = Promise.resolve();
export function saveProject(document: Project) {
  const task = pending
    .catch(() => {})
    .then(() => bridge<void>("save_project", { document }));
  pending = task;
  return task;
}
