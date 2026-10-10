import { t } from "../i18n";
import type { ModelConnection } from "./types";
export const inputLabels = {
  image: "图片",
  audio: "音频",
  video: "视频",
  document: "PDF",
};
export function supportsInput(model: ModelConnection, kind: string) {
  return kind === "text" || !!model.inputs[kind as keyof typeof inputLabels];
}
export function inputSummary(model: ModelConnection) {
  return [
    "文本",
    ...Object.entries(inputLabels)
      .filter(([kind]) => supportsInput(model, kind))
      .map(([, label]) => label),
  ]
    .map((label) => t(label))
    .join(" · ");
}
