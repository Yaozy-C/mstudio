import type { ModelConnection } from "./types";
import { protocolSupports } from "./inputCapabilities";

// Exact official model IDs only. Compatible endpoints may expose different capabilities.
// Verified against provider documentation on 2026-09-22.
export function verifiedInputs(profile: ModelConnection) {
  let host: string;
  try {
    host = new URL(profile.endpoint).hostname;
  } catch {
    return null;
  }
  const model = profile.model.trim();
  if (host === "api.deepseek.com" && model === "deepseek-flash")
    return {
      inputs: { image: true, audio: false, video: false, document: false },
      source: "https://api-docs.deepseek.com/guides/vision/",
      label:
        "DeepSeek Flash：文字、图片；官方未列出原生音频、视频和 PDF 输入。",
    };
  if (host === "api.openai.com" && model === "gpt-6-astra")
    return {
      inputs: { image: true, audio: false, video: false, document: true },
      source: "https://developers.openai.com/api/docs/models/gpt-6-astra",
      label:
        "GPT-6 Astra：文字、图片；PDF 通过文件输入读取，不支持原生音频、视频。",
    };
  if (
    host === "generativelanguage.googleapis.com" &&
    ["gemini-3.7-flash", "gemini-3.8-flash"].includes(model)
  )
    return {
      inputs: { image: true, audio: true, video: true, document: true },
      source: `https://ai.google.dev/gemini-api/docs/models/${model}`,
      label: `${model}：文字、图片、音频、视频和 PDF 输入；输出为文字。`,
    };
  return null;
}
export function recommendedInputs(profile: ModelConnection) {
  const verified = verifiedInputs(profile);
  if (!verified) return null;
  return Object.fromEntries(
    Object.entries(verified.inputs).map(([kind, supported]) => [
      kind,
      supported && protocolSupports(profile.adapter, kind),
    ]),
  ) as ModelConnection["inputs"];
}
