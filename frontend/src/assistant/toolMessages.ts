import messages from "./toolMessages.json";

// Translate only known application diagnostics at the Agent boundary. Shared
// UI validators remain localized; user-authored content and vendor text stay intact.
function translate(text: string): string {
  const known = (messages as Record<string, string>)[text];
  if (known) return known;
  const patterns: [RegExp, string][] = [
    [/^不支持的调色参数：(.+)$/, "Unsupported grade parameter: $1"],
    [/^不支持的脚本字段：(.+)$/, "Unsupported screenplay field: $1"],
    [/^不支持的镜头字段：(.+)$/, "Unsupported shot field: $1"],
    [/^调色参数 (.+) 超出范围$/, "Visual parameter $1 outside allowed range"],
    [/^(.+) 超出允许范围$/, "$1 outside allowed range"],
    [/^(.+) 控制点无效$/, "Invalid $1 control points"],
    [/^(.+)必须为布尔值$/, "$1 must be a boolean"],
    [
      /^(.+)必须是不重复的段落 ID 数组$/,
      "$1 must be an array of unique paragraph IDs",
    ],
    [
      /^「(.+)」素材与计划时长不匹配，请先调整镜头时长$/,
      "Media for '$1' does not match planned duration; adjust shot timing first",
    ],
    [
      /^请使用用户选择的模型 (.+)；省略 mediaModelId 可沿用本轮选择。$/,
      "Use the user-selected model $1; omit mediaModelId to retain the selection.",
    ],
  ];
  for (const [pattern, replacement] of patterns)
    if (pattern.test(text)) return text.replace(pattern, replacement);
  const numeric = /^(时间线位置|速度|源素材偏移秒数)必须为有限数值$/.exec(text);
  if (numeric) {
    const name = {
      时间线位置: "Timeline start",
      速度: "Speed",
      源素材偏移秒数: "Source offset",
    }[numeric[1]];
    return `${name} must be a finite number`;
  }
  return text;
}
export function modelToolError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  try {
    const issue = JSON.parse(raw);
    if (
      issue &&
      typeof issue.code === "string" &&
      typeof issue.message === "string"
    ) {
      // Keep outcome and diagnostic details, not localized UI headings/recovery copy.
      return JSON.stringify({
        code: issue.code,
        message: translate(issue.details || issue.message),
        retryable: issue.retryable,
        httpStatus: issue.httpStatus,
        stage: issue.stage,
        outcome: issue.outcome,
      });
    }
  } catch {
    /* Plain application/provider diagnostic. */
  }
  return translate(raw);
}
