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
  ).toBeUndefined();
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
  expect(
    contextPolicy({ ...profile, model: "new-model" }).window,
  ).toBeUndefined();
  expect(
    contextPolicy({ ...profile, adapter: "codex" }).window,
  ).toBeUndefined();
});

test("unknown models require capacity without inventing protocol output caps", () => {
  for (const adapter of [
    "anthropic-native",
    "gemini-native",
    "openai-compatible",
  ] as const) {
    const profile = {
      ...newConnection(),
      adapter,
      endpoint: "https://proxy.example",
      model: "unknown",
    };
    expect(automaticLimits(profile)).toBeNull();
    expect(contextPolicy(profile)).toMatchObject({
      output: null,
      threshold: null,
    });
    expect(contextPolicyIssue(profile)).toContain("刷新容量或手动填写");
    const manual = {
      ...profile,
      contextWindowMode: "manual" as const,
      contextWindow: 200000,
    };
    expect(contextPolicyIssue(manual)).toBe(
      adapter === "anthropic-native"
        ? "此 Anthropic 型号未公布默认输出上限，请手动填写输出上限"
        : "",
    );
    expect(contextPolicy(manual)).toMatchObject({
      window: 200000,
      output: null,
    });
    expect(contextPolicyIssue({ ...manual, contextWindow: null })).toContain(
      "填写手动上下文",
    );
    expect(contextPolicy({ ...manual, maxOutputTokens: 7777 }).output).toBe(
      7777,
    );
    expect(contextPolicyIssue({ ...manual, maxOutputTokens: 7777 })).toBe("");
  }
  expect(contextPolicyIssue({ ...newConnection(), adapter: "codex" })).toBe("");
});

test("service metadata never revives old inferred output defaults", () => {
  const profile: ModelConnection = {
    ...newConnection(),
    endpoint: "https://proxy.example",
    model: "unknown",
    adapter: "gemini-native",
    modelLimits: {
      endpoint: "https://proxy.example",
      model: "models/unknown",
      contextWindow: 500000,
      maxOutputTokens: 32768,
      defaultMaxTokens: 65536,
      source: "service",
    },
  };
  expect(contextPolicy(profile)).toMatchObject({
    window: 500000,
    output: null,
  });
  expect(contextPolicyIssue(profile)).toBe("");
  expect(contextPolicy({ ...profile, maxOutputTokens: 30000 }).output).toBe(
    30000,
  );
  expect(contextPolicyIssue({ ...profile, maxOutputTokens: 33000 })).toContain(
    "公布的输出能力",
  );
  profile.modelLimits!.contextWindow = 1;
  expect(automaticLimits(profile)).toBeNull();
});
