import { expect, test } from "bun:test";
import { newConnection } from "./types";
import { connectionIssue } from "./connectionIssue";
test("Google connections require the native protocol and base URL", () => {
  const p = {
    ...newConnection(),
    endpoint: "https://generativelanguage.googleapis.com/v1beta/openai",
  };
  expect(connectionIssue(p)).toContain("Gemini 原生协议");
  expect(connectionIssue({ ...p, adapter: "gemini-native" })).toContain(
    "不附加",
  );
  expect(
    connectionIssue({
      ...p,
      adapter: "gemini-native",
      endpoint: "https://generativelanguage.googleapis.com/",
    }),
  ).toBe("");
  expect(
    connectionIssue({ ...p, endpoint: "https://api.example.com/v1" }),
  ).toBe("");
});
