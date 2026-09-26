import { inputHint } from "./inputHint";
// Stable public codes. Keep transport errors separate from the job's outcome.
export const errorCatalog = {
  INVALID_INPUT: [
    "生成设置不符合要求",
    "检查模型支持的参数和参考素材，再开始生成。",
  ],
  AUTH_REQUIRED: ["服务连接验证失败", "打开模型设置，检查登录状态或连接凭据。"],
  ACCESS_DENIED: ["没有权限使用此服务", "检查账户权限，或选择其他可用模型。"],
  QUOTA_EXCEEDED: ["服务额度不足", "到服务商检查额度，再重试。"],
  RATE_LIMITED: ["请求过于频繁", "稍后重试，或检查服务商的并发限制。"],
  SERVICE_UNAVAILABLE: ["生成服务暂时不可用", "稍后重新查询原任务。"],
  NETWORK_ERROR: ["暂时无法连接服务", "检查网络后重试。"],
  SUBMISSION_UNKNOWN: [
    "尚不能确认是否提交成功",
    "请先在服务商核查原任务，避免重复生成。",
  ],
  GENERATION_FAILED: ["本次生成未完成", "保留了原设置，可以调整后重新生成。"],
  CONTENT_REJECTED: [
    "内容未通过服务审核",
    "调整生成描述或参考素材后重新生成。",
  ],
  JOB_NOT_FOUND: ["暂时无法找到原任务", "到服务商核查任务记录后再处理。"],
  JOB_CANCEL_FAILED: ["任务尚未取消", "重新查询任务状态，或到服务商取消。"],
  JOB_CANCELLED: ["任务已取消", "可以保留原设置重新生成。"],
  JOB_SYNC_FAILED: ["暂时无法获取任务进度", "重新查询原任务；无需重新生成。"],
  RESULT_IMPORT_FAILED: [
    "生成结果尚未收取成功",
    "重试收取已有结果，无需重新生成。",
  ],
  MODEL_UNAVAILABLE: [
    "所选模型不可用",
    "打开模型设置检查连接，或选择其他模型。",
  ],
  VALIDATION_FAILED: ["请检查当前设置", "修改提示的问题后再继续。"],
  SAVE_FAILED: ["项目尚未保存成功", "检查存储空间和目录权限，再重试保存。"],
  EXPORT_FAILED: ["成片导出未完成", "检查素材是否可用，再重试导出。"],
  EXPORT_SAVE_FAILED: [
    "成片尚未保存到所选位置",
    "成片仍然保留，请重新选择保存位置。",
  ],
  ASSET_IMPORT_FAILED: [
    "素材导入未完成",
    "检查文件是否可读取，重新选择失败的文件。",
  ],
  ASSET_OPERATION_FAILED: [
    "素材操作未完成",
    "检查文件和存储目录后重试此操作。",
  ],
  CHAT_FAILED: ["回答未完成", "原消息和附件已保留，可以重试。"],
  STORAGE_FAILED: ["存储操作未完成", "检查存储设备、空间与目录权限后重试。"],
  OPERATION_FAILED: ["操作未完成", "请查看错误详情，处理后再重试。"],
} as const;
export type ErrorCode = keyof typeof errorCatalog;
export type AppIssue = {
  code: ErrorCode;
  message: string;
  recovery: string;
  retryable: boolean;
  details?: string;
  httpStatus?: number;
  stage?: string;
  outcome?: "rejected" | "unknown" | "failed" | "cancelled";
};
export function redactDetails(value: string): string {
  return value
    .replace(/https?:\/\/[^\s"<>]+/gi, "[服务地址已隐藏]")
    .replace(/data:[^\s"<>]+/gi, "[媒体数据已隐藏]")
    .replace(/\b(Bearer|Key)\s+[^\s",;]+/gi, "$1 [已隐藏]")
    .replace(
      /((?:api[_-]?key|token|password|authorization|secret)["']?\s*[:=]\s*)["']?[^\s,;"'}]+/gi,
      "$1[已隐藏]",
    )
    .replace(/\bsk-[\w-]+/g, "[已隐藏]")
    .slice(0, 2500);
}
export function issue(
  code: ErrorCode,
  details?: string,
  extra: Partial<AppIssue> = {},
): AppIssue {
  const [message, recovery] = errorCatalog[code];
  return {
    code,
    message,
    recovery,
    ...(code === "INVALID_INPUT" ? inputHint(details) : {}),
    retryable: false,
    ...extra,
    details: details ? redactDetails(details) : undefined,
  };
}
function rawText(value: unknown): string {
  if (value instanceof Error) return value.message;
  if (typeof value === "string") return value.replace(/^Error:\s*/, "");
  try {
    return JSON.stringify(value) ?? "";
  } catch {
    return "无法读取错误详情";
  }
}
export function normalizeError(
  value: unknown,
  fallback: ErrorCode = "OPERATION_FAILED",
): AppIssue {
  const raw = rawText(value);
  try {
    const data =
      typeof value === "object" && !(value instanceof Error)
        ? value
        : JSON.parse(raw.slice(raw.indexOf("{")));
    if (
      data &&
      typeof data === "object" &&
      "code" in data &&
      Object.hasOwn(errorCatalog, data.code)
    ) {
      const code = data.code as ErrorCode;
      const result = issue(
        code,
        typeof data.details === "string" ? data.details : undefined,
        {
          retryable: data.retryable === true,
          httpStatus:
            typeof data.httpStatus === "number" ? data.httpStatus : undefined,
          stage: typeof data.stage === "string" ? data.stage : undefined,
          outcome: ["rejected", "unknown", "failed", "cancelled"].includes(
            data.outcome,
          )
            ? data.outcome
            : undefined,
        },
      );
      if (code === "VALIDATION_FAILED") {
        const hint =
          typeof data.message === "string" ? data.message : data.details;
        if (
          typeof hint === "string" &&
          hint.length < 250 &&
          /[\u4e00-\u9fff]/.test(hint) &&
          !/[{}]|Error|panic|stack/i.test(hint)
        )
          result.message = redactDetails(hint);
      }
      return result;
    }
  } catch {
    /* Legacy project errors are display-only; never infer job outcomes. */
  }
  const http = /(?:HTTP\s*|status(?: code)?[:= ]+)(\d{3})/i.exec(raw)?.[1];
  const codes: Record<string, ErrorCode> = {
    "400": "INVALID_INPUT",
    "422": "INVALID_INPUT",
    "401": "AUTH_REQUIRED",
    "403": "ACCESS_DENIED",
    "402": "QUOTA_EXCEEDED",
    "429": "RATE_LIMITED",
  };
  let code = http
    ? (codes[http] ?? (+http >= 500 ? "SERVICE_UNAVAILABLE" : fallback))
    : fallback;
  if (
    !http &&
    /network|failed to fetch|connection|timed? ?out|网络|连接中断/i.test(raw)
  )
    code = "NETWORK_ERROR";
  if (!http && /API Key|登录|凭据/.test(raw)) code = "AUTH_REQUIRED";
  if (!http && /模型已移除|请选择.*模型|请先选择.*模型|连接生成服务/.test(raw))
    code = "MODEL_UNAVAILABLE";
  const result = issue(code, raw, { httpStatus: http ? +http : undefined });
  // Local validation messages are actionable, already authored for users.
  if (
    fallback === "VALIDATION_FAILED" &&
    code === fallback &&
    raw.length < 250 &&
    /[\u4e00-\u9fff]/.test(raw) &&
    !/[{}]|Error|panic|stack/i.test(raw)
  )
    result.message = redactDetails(raw);
  return result;
}
export function errorText(
  value: unknown,
  fallback: ErrorCode,
  extra: Partial<AppIssue> = {},
): string {
  return JSON.stringify({ ...normalizeError(value, fallback), ...extra });
}
