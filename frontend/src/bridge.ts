import { mergeValue } from "./domain/merge";
import { commandError } from "./errors/commands";
import { invoke, convertFileSrc, isTauri } from "@tauri-apps/api/core";
import type { Project, ProjectEntry, ProjectSummary } from "./model";
const previewEntries = (): ProjectEntry[] =>
  JSON.parse(localStorage.getItem("mstudio-preview") || "[]");
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
    return previewEntries().map(
      ({ id, name, updated, document }) =>
        ({
          id,
          name,
          updated,
          width: document.width,
          height: document.height,
          clipCount: document.clips.length,
          preview:
            document.assets.find((a) => a.preview && !a.missing)?.preview ?? "",
        }) satisfies ProjectSummary,
    ) as T;
  if (command === "save_project" || command === "create_project") {
    let p = args!.document as Project;
    const entries = previewEntries();
    const exists = entries.some((e) => e.id === p.id);
    if (command === "save_project" && !exists) throw new Error("项目已删除");
    if (command === "create_project" && exists) throw new Error("项目已存在");
    if (command === "save_project")
      p = mergeValue(
        args!.base,
        p,
        entries.find((e) => e.id === p.id)!.document,
      ) as Project;
    const next = [
      { id: p.id, name: p.name, updated: Date.now() / 1000, document: p },
      ...entries.filter((e) => e.id !== p.id),
    ];
    localStorage.setItem("mstudio-preview", JSON.stringify(next));
    return p as T;
  }
  if (command === "get_project")
    return previewEntries().find((p) => p.id === args!.projectId)!
      .document as T;
  if (command === "delete_project") {
    const entries = previewEntries();
    localStorage.setItem(
      "mstudio-preview",
      JSON.stringify(entries.filter((e) => e.id !== args!.id)),
    );
    return undefined as T;
  }
  if (command === "media_model_catalog") return [] as T;
  if (command === "list_global_assets") return [] as T;
  throw new Error(
    "此功能需要在 Mstudio 桌面应用中使用。浏览器仅提供界面预览。",
  );
}
let pending: Promise<unknown> = Promise.resolve();
export function saveProject(document: Project, base: Project) {
  const task = pending
    .catch(() => {})
    .then(() => bridge<Project>("save_project", { document, base }));
  pending = task;
  return task;
}
