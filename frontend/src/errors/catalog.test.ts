import { expect, test } from "bun:test";
import { errorText, issue, normalizeError, redactDetails } from "./catalog";
import { commandError } from "./commands";

test("assistant-ui error envelopes preserve readable details and issue metadata", () => {
  const inner = issue("CHAT_FAILED", "Codex: 连接超时", { stage: "chat" });
  const result = normalizeError(
    { code: "unknown", message: JSON.stringify(inner) },
    "CHAT_FAILED",
  );
  expect(result.code).toBe("CHAT_FAILED");
  expect(result.details).toBe("Codex: 连接超时");
  expect(result.stage).toBe("chat");
  expect(
    normalizeError({ code: "unknown", message: "普通错误" }, "CHAT_FAILED")
      .details,
  ).toBe("普通错误");
});

test("reference upload transport failures point to the network and hide URLs", () => {
  const value = normalizeError(
    commandError(
      "upload_references",
      "error sending request for url (https://private.test/upload?token=secret)",
    ),
  );
  expect(value.code).toBe("NETWORK_ERROR");
  expect(value.recovery).toBe("检查网络后重试。");
  expect(value.details).not.toContain("private.test");
  expect(value.details).not.toContain("secret");
  expect(value.outcome).toBeUndefined();
  const typed = normalizeError(
    commandError("upload_references", {
      code: "NETWORK_ERROR",
      stage: "reference_upload_transfer",
      details: "服务请求超时",
      retryable: true,
    }),
  );
  expect(typed.stage).toBe("reference_upload_transfer");
  expect(typed.retryable).toBe(true);
  expect(
    normalizeError(commandError("import_media", "Permission denied")).code,
  ).toBe("ASSET_IMPORT_FAILED");
});

test("typed rejection survives native Error, persisted strings and contextual prefixes", () => {
  const wire = JSON.stringify({
    code: "INVALID_INPUT",
    stage: "submit",
    outcome: "rejected",
    httpStatus: 422,
    details: "body.duration: unsupported",
    retryable: false,
  });
  const normalized = normalizeError(
    new Error(commandError("submit_job", wire)),
  );
  expect(normalized.code).toBe("INVALID_INPUT");
  expect(normalized.outcome).toBe("rejected");
  expect(normalized.details).toContain("duration");
  expect(
    normalizeError(
      `退出前保存失败：Error: ${errorText("disk full", "SAVE_FAILED")}`,
    ).code,
  ).toBe("SAVE_FAILED");
});
test("legacy 422 explains the issue without guessing remote job outcome", () => {
  const value = normalizeError(
    "进度同步或结果导入失败，正在自动重试：生成服务返回 HTTP 422；请检查模型参数、余额和连接",
  );
  expect(value.code).toBe("INVALID_INPUT");
  expect(value.message).not.toContain("余额");
  expect(value.outcome).toBeUndefined();
});
test("unknown failures keep readable context and details never leak common credentials", () => {
  const value = normalizeError("SQLITE_IOERR: failed", "SAVE_FAILED");
  expect(value.message).toBe("项目尚未保存成功");
  expect(value.details).toContain("SQLITE_IOERR");
  const hidden = redactDetails(
    'Authorization: Bearer secret-value api_key="another-secret" https://private.test/?token=secret sk-private-token data:image/png;base64,AAAA',
  );
  for (const secret of [
    "secret-value",
    "another-secret",
    "private.test",
    "sk-private-token",
    "AAAA",
  ])
    expect(hidden).not.toContain(secret);
});
test("local validation remains actionable through serialization", () => {
  expect(
    normalizeError(
      errorText("当前模型的视频时长须为 5–15 秒", "VALIDATION_FAILED"),
    ).message,
  ).toContain("5–15");
  expect(normalizeError({ code: "__proto__", message: "bad" }).code).toBe(
    "OPERATION_FAILED",
  );
  expect(
    normalizeError(
      issue("SUBMISSION_UNKNOWN", "timeout", { outcome: "unknown" }),
    ).outcome,
  ).toBe("unknown");
});

test("provider field constraints become Chinese guidance without changing the error outcome", () => {
  const problem = normalizeError(
    JSON.stringify({
      code: "INVALID_INPUT",
      httpStatus: 422,
      details:
        "body.reference_image_urls: The aspect ratio of the image should be between 0.4 and 2.5.",
    }),
  );
  expect(problem.message).toBe("参考图片的宽高比不符合要求");
  expect(problem.recovery).toContain("0.4～2.5");
  expect(problem.outcome).toBeUndefined();
});

test("cancelled chat is a stopped operation, not a failed model request", () => {
  const stopped = normalizeError(
    "已停止回答；已输出内容和已完成操作保留",
    "CHAT_FAILED",
  );
  expect(stopped.code).toBe("CHAT_STOPPED");
  expect(stopped.outcome).toBe("cancelled");
  expect(normalizeError("模型输出被截断", "CHAT_FAILED").code).toBe(
    "CHAT_FAILED",
  );
});

test("local preview worker failures never become network troubleshooting", () => {
  const value = normalizeError(
    new Error(
      commandError(
        "native_preview_status",
        "预览进程已退出或连接中断：connection closed",
      ),
    ),
  );
  expect(value.code).toBe("PREVIEW_FAILED");
  expect(value.recovery).toContain("重新加载预览");
});

test("typed error codes do not change when diagnostic wording mentions credentials or models", () => {
  for (const details of [
    "API Key 登录 凭据",
    "请选择模型",
    "Credentials and selected model",
    "随意修改的文案",
  ]) {
    const value = normalizeError({
      code: "SAVE_FAILED",
      details,
      retryable: false,
    });
    expect(value.code).toBe("SAVE_FAILED");
  }
  expect(normalizeError("日志中提到了 API Key", "SAVE_FAILED").code).toBe(
    "SAVE_FAILED",
  );
  const stopped = normalizeError({
    code: "CHAT_STOPPED",
    details: "arbitrary text",
    outcome: "cancelled",
  });
  expect(stopped.code).toBe("CHAT_STOPPED");
  expect(stopped.outcome).toBe("cancelled");
});
