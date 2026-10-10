import { expect, test } from "bun:test";
import { newConnection } from "./types";
import { recommendedInputs, verifiedInputs } from "./verifiedInputs";
test("official capability suggestions are independent of protocol", () => {
  const base = newConnection();
  const ds = {
    ...base,
    endpoint: "https://api.deepseek.com",
    model: "deepseek-flash",
  };
  expect(recommendedInputs(ds)).toEqual({
    image: true,
    audio: false,
    video: false,
    document: false,
  });
  const gemini = {
    ...base,
    endpoint: "https://generativelanguage.googleapis.com",
    model: "gemini-3.7-flash",
    adapter: "gemini-native" as const,
  };
  expect(recommendedInputs(gemini)).toEqual({
    image: true,
    audio: true,
    video: true,
    document: true,
  });
  const gpt = {
    ...base,
    endpoint: "https://api.openai.com/v1",
    model: "gpt-6-astra",
    adapter: "openai-responses" as const,
  };
  expect(recommendedInputs(gpt)).toEqual({
    image: true,
    audio: false,
    video: false,
    document: true,
  });
  expect(
    recommendedInputs({ ...gpt, adapter: "openai-compatible" })?.document,
  ).toBe(true);
});
test("custom services and unknown models do not inherit official capabilities", () => {
  const base = {
    ...newConnection(),
    endpoint: "https://proxy.example/v1",
    model: "gpt-6-astra",
  };
  expect(verifiedInputs(base)).toBeNull();
  expect(
    verifiedInputs({
      ...base,
      endpoint: "https://api.openai.com",
      model: "unknown",
    }),
  ).toBeNull();
});
