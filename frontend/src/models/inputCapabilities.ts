import type { ModelConnection } from "./types";
export const inputLabels = {
  image: "图片",
  audio: "音频",
  video: "视频",
  document: "PDF",
};
export function protocolSupports(
  adapter: ModelConnection["adapter"],
  kind: string,
) {
  if (kind === "text" || kind === "image") return true;
  if (kind === "audio")
    return ["gemini-native", "openai-compatible"].includes(adapter);
  if (kind === "video") return adapter === "gemini-native";
  if (kind === "document")
    return ["gemini-native", "anthropic-native", "openai-responses"].includes(
      adapter,
    );
  return false;
}
export function supportsInput(model: ModelConnection, kind: string) {
  return (
    kind === "text" ||
    (protocolSupports(model.adapter, kind) &&
      !!model.inputs[kind as keyof typeof inputLabels])
  );
}
export function inputSummary(model: ModelConnection) {
  return [
    "文本",
    ...Object.entries(inputLabels)
      .filter(([kind]) => supportsInput(model, kind))
      .map(([, label]) => label),
  ].join(" · ");
}
