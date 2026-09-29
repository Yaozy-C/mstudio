import type { ScriptParagraph } from "./types";
import { patchScript } from "./scriptOperations";
export function writeScript(
  old: ScriptParagraph[] = [],
  screenplay: Record<string, unknown>,
) {
  const mode = screenplay.scriptMode ?? "merge";
  if (mode !== "merge" && mode !== "replace")
    throw new Error("scriptMode 必须为 merge 或 replace");
  if (mode === "replace") {
    if (!Array.isArray(screenplay.script))
      throw new Error("整篇替换必须提供完整 script 数组");
    if (
      screenplay.removeParagraphIds !== undefined ||
      screenplay.paragraphOrder !== undefined
    )
      throw new Error("整篇替换直接使用 script 顺序，不得混用删除或排序");
    for (const item of screenplay.script) {
      if (
        !item ||
        ["title", "action", "dialogue", "sound"].some(
          (k) => typeof item[k] !== "string",
        ) ||
        item.duration === undefined
      )
        throw new Error(
          "整篇替换的每段必须包含 id/title/action/dialogue/sound/duration",
        );
    }
    return patchScript([], screenplay.script);
  }
  let next =
    screenplay.script === undefined ? old : patchScript(old, screenplay.script);
  const ids = (value: unknown, label: string) => {
    if (
      !Array.isArray(value) ||
      value.some((id) => typeof id !== "string") ||
      new Set(value).size !== value.length
    )
      throw new Error(`${label}必须是不重复的段落 ID 数组`);
    return value as string[];
  };
  if (screenplay.removeParagraphIds !== undefined) {
    const remove = ids(screenplay.removeParagraphIds, "删除列表");
    if (remove.some((id) => !old.some((s) => s.id === id)))
      throw new Error("要删除的段落不存在，请重新读取脚本");
    if (
      Array.isArray(screenplay.script) &&
      screenplay.script.some((s) => remove.includes(s.id))
    )
      throw new Error("不能同时修改和删除同一段");
    next = next.filter((s) => !remove.includes(s.id));
  }
  if (screenplay.paragraphOrder !== undefined) {
    const order = ids(screenplay.paragraphOrder, "排序列表");
    if (
      order.length !== next.length ||
      order.some((id) => !next.some((s) => s.id === id))
    )
      throw new Error("排序必须包含操作后全部段落 ID，且每个只出现一次");
    next = order.map((id) => next.find((s) => s.id === id)!);
  }
  return next;
}
