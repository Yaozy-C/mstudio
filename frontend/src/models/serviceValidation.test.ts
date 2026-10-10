import { expect, test } from "bun:test";
import { validateService } from "./serviceValidation";
import type { ServiceConnection } from "./connectionStore";
const initial: ServiceConnection = {
  id: "s",
  name: "Service",
  kind: "dashscope",
  endpoint: "https://old.example.com",
  hasKey: true,
  modelCount: 0,
};
test("changing a keyed address requires reentry or explicit removal, not a masked saved value", () => {
  const changed = { ...initial, endpoint: "https://new.example.com" };
  expect(validateService(changed, initial, "", false)).toEqual({
    key: "地址已修改，请重新输入 API Key",
  });
  expect(validateService(changed, initial, "replacement", false)).toEqual({});
  expect(validateService(changed, initial, "", true)).toEqual({});
  expect(
    validateService(
      { ...initial, endpoint: initial.endpoint + "/" },
      initial,
      "",
      false,
    ),
  ).toEqual({});
});
test("invalid settings belong to their fields and local unauthenticated services remain allowed", () => {
  const invalid = { ...initial, name: " ", endpoint: "invalid", hasKey: false };
  expect(
    validateService(invalid, { ...initial, hasKey: false }, "bad\nkey", false),
  ).toEqual({
    name: "请填写连接名称",
    endpoint: "请填写有效的服务地址",
    key: "API Key 格式无效",
  });
  expect(
    validateService(
      { ...initial, endpoint: "http://127.0.0.1:8000", hasKey: false },
      { ...initial, hasKey: false },
      "",
      false,
    ),
  ).toEqual({});
  expect(
    validateService(
      { ...initial, endpoint: "http://remote.example.com" },
      initial,
      "replacement",
      false,
    ).endpoint,
  ).toBeTruthy();
});
