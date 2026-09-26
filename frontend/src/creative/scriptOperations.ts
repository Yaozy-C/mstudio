import type { ScriptParagraph } from "./types";
export function patchScript(
  old: ScriptParagraph[] = [],
  value: unknown,
): ScriptParagraph[] {
  if (!Array.isArray(value) || value.length > 200)
    throw new Error("脚本段落格式无效");
  const result = [...old],
    ids = new Set<string>();
  for (const item of value) {
    if (
      !item ||
      typeof item !== "object" ||
      typeof item.id !== "string" ||
      !item.id.trim() ||
      item.id.length > 100 ||
      ids.has(item.id)
    )
      throw new Error("脚本段落 ID 无效或重复");
    ids.add(item.id);
    const index = result.findIndex((s) => s.id === item.id);
    const next: ScriptParagraph = {
      id: item.id,
      title: "",
      action: "",
      dialogue: "",
      sound: "",
      ...(index < 0 ? {} : result[index]),
    };
    for (const key of [
      "title",
      "action",
      "onScreenText",
      "dialogue",
      "sound",
    ] as const) {
      if (item[key] === undefined) continue;
      if (typeof item[key] !== "string" || item[key].length > 6000)
        throw new Error("脚本文字无效或过长");
      next[key] = item[key];
    }
    if (item.duration !== undefined) {
      if (
        typeof item.duration !== "number" ||
        !Number.isFinite(item.duration) ||
        item.duration < 0.01 ||
        item.duration > 3600
      )
        throw new Error("脚本段落时长须为 0.01–3600 秒");
      next.duration = item.duration;
    }
    if (index < 0) result.push(next);
    else result[index] = next;
  }
  if (result.length > 200) throw new Error("脚本段落不能超过 200 个");
  return result;
}
