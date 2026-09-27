import type { ScriptParagraph } from "../creative/types";
const keys = [
  "title",
  "duration",
  "action",
  "onScreenText",
  "dialogue",
  "sound",
] as const;
export function inspectScript(
  script: ScriptParagraph[],
  args: Record<string, unknown>,
  full: boolean,
) {
  const ids = Array.isArray(args.paragraphIds)
    ? args.paragraphIds.slice(0, 12)
    : [];
  const fields = Array.isArray(args.scriptFields)
    ? args.scriptFields
    : full
      ? keys
      : ["title", "duration"];
  const offset =
    typeof args.offset === "number" && Number.isFinite(args.offset)
      ? Math.max(0, Math.floor(args.offset))
      : 0;
  const textOffset =
    typeof args.textOffset === "number" && Number.isFinite(args.textOffset)
      ? Math.max(0, Math.floor(args.textOffset))
      : 0;
  const selected = ids.length
    ? script.filter((s) => ids.includes(s.id))
    : script;
  return {
    script: selected
      .slice(offset, offset + 10)
      .map((s): { id: string; [field: string]: unknown } => ({
        id: s.id,
        ...Object.fromEntries(
          keys
            .filter((k) => fields.includes(k))
            .map((k) => {
              const value = k === "duration" ? (s.duration ?? 5) : (s[k] ?? "");
              return [
                k,
                typeof value === "number" || k === "title"
                  ? value
                  : {
                      text: value.slice(textOffset, textOffset + 1000),
                      nextTextOffset:
                        value.length > textOffset + 1000
                          ? textOffset + 1000
                          : null,
                    },
              ];
            }),
        ),
      })),
    scriptCount: script.length,
    matchedScriptCount: selected.length,
    missingParagraphIds: ids.filter((id) => !script.some((s) => s.id === id)),
    omittedScriptFields: keys.filter((k) => !fields.includes(k)),
    nextScriptOffset: selected.length > offset + 10 ? offset + 10 : null,
  };
}
