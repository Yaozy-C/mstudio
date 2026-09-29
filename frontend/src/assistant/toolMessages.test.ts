import { expect, test } from "bun:test";
import { failure } from "../errors/failure";
import { modelToolError } from "./toolMessages";

test("agent diagnostics are English without changing shared UI errors or outcome", () => {
  const error = new Error("工程已变化，请重新 inspect 后使用最新 revision");
  expect(modelToolError(error)).toBe(
    "Project changed; inspect again and use the latest revision",
  );
  expect(error.message).toContain("工程已变化");
  const structured = failure("VALIDATION_FAILED", "参考素材不能重复");
  const result = JSON.parse(modelToolError(structured));
  expect(result.code).toBe("VALIDATION_FAILED");
  expect(result.message).toBe("Reference assets cannot be duplicated");
  expect(modelToolError(new Error("不支持的镜头字段：secret"))).toBe(
    "Unsupported shot field: secret",
  );
});

test("unknown diagnostics and quoted project names are not rewritten", () => {
  expect(modelToolError("供应商原始错误：保持中文证据")).toBe(
    "供应商原始错误：保持中文证据",
  );
  expect(
    modelToolError("「中文片名」素材与计划时长不匹配，请先调整镜头时长"),
  ).toContain("中文片名");
});
