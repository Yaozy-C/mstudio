import { expect, test } from "bun:test";
import {
  automaticLimits,
  contextPolicy,
  contextPolicyIssue,
} from "./contextLimits";
import { newConnection, type ModelConnection } from "./types";

const gemini = () => ({
  ...newConnection(),
  endpoint: "https://generativelanguage.googleapis.com",
  adapter: "gemini-native" as const,
  model: "gemini-4-argon",
});
test("new models resolve verified capacity instead of inheriting 32K", () => {
  const profile = gemini();
  expect(profile.contextWindow).toBeNull();
  expect(contextPolicy(profile)).toMatchObject({
    window: 1048576,
    output: 32768,
    headroom: 65536,
    threshold: 838860,
  });
  expect(contextPolicyIssue(profile)).toBe("");
  expect(
    automaticLimits({ ...profile, endpoint: "https://proxy.example" })?.source,
  ).toBe("fallback");
});
test("manual override survives automatic discovery and impossible budgets are actionable", () => {
  const profile = {
    ...gemini(),
    contextWindowMode: "manual" as const,
    contextWindow: 200000,
  };
  expect(contextPolicy(profile).window).toBe(200000);
  expect(contextPolicy(profile).threshold).toBe(101696);
  expect(contextPolicyIssue({ ...profile, contextWindow: 8192 })).toContain(
    "输出预留",
  );
  expect(contextPolicyIssue({ ...profile, maxOutputTokens: -1 })).toContain(
    "输出上限",
  );
});
test("service capacities are scoped to model and endpoint; Codex stays runtime-owned", () => {
  const profile: ModelConnection = gemini();
  profile.modelLimits = {
    ...automaticLimits(profile)!,
    contextWindow: 500000,
    source: "service",
  };
  expect(contextPolicy(profile).window).toBe(500000);
  expect(contextPolicy({ ...profile, model: "new-model" }).window).toBe(262144);
  expect(
    contextPolicy({ ...profile, adapter: "codex" }).window,
  ).toBeUndefined();
});
