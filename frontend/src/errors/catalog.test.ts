import { expect, test } from "bun:test";
import { errorText, issue, normalizeError, redactDetails } from "./catalog";
import { commandError } from "./commands";

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
