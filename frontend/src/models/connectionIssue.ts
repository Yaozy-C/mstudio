import type { ModelConnection } from "./types";
export function connectionIssue(profile: ModelConnection) {
  let url: URL;
  try {
    url = new URL(profile.endpoint);
  } catch {
    return "";
  }
  if (url.hostname !== "generativelanguage.googleapis.com") return "";
  if (profile.adapter !== "gemini-native")
    return "Google Gemini 请使用 Gemini 原生协议，确保 Agent 调用工具后能继续回答。";
  if (url.pathname.replaceAll("/", ""))
    return "Gemini 原生地址填写 https://generativelanguage.googleapis.com，不附加 /v1beta 或 /openai。";
  return "";
}
