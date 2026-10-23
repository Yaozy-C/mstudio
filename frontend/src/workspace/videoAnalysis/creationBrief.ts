import { newProject } from "../../model";
import { t } from "../../i18n";
import type { CreativeTask } from "../../creative/aiTasks";
import { timecode, type AnalysisState } from "./types";

const craft = {
  idea: "创意内核",
  visuals: "画面与美术",
  camera: "镜头与剪辑",
  language: "话语与文字",
  sound: "音乐与声音",
} as const;
function stamp(seconds: number) {
  const ms = Math.round(Math.max(0, seconds) * 1000);
  return `${timecode(ms / 1000)}.${String(ms % 1000).padStart(3, "0")}`;
}
export function analysisCreationTask(state: AnalysisState): CreativeTask {
  if (!state.result) throw new Error(t("请先完成视频拆解"));
  const title = state.work?.title || state.video?.name || t("参考视频");
  const result = state.result;
  const text = [
    t("我想借鉴《{title}》，做一条自己的新视频。", { title }),
    t(
      "请结合下面的拆解，提炼可迁移的创意机制，转成适合我的主题或商品的创作方案，再推进脚本和制作。缺少主题、商品、受众等关键资料时先问我，不要把参考片的信息当成我的事实。",
    ),
    state.question.trim() ? `${t("当前关注点")}：${state.question.trim()}` : "",
    `## ${t("参考作品")}`,
    title,
    state.work?.credit ? `${t("创作者")}：${state.work.credit}` : "",
    state.work?.source || state.link
      ? `${t("作品来源")}：${state.work?.source || state.link}`
      : "",
    t("以下是已完成的拆解，作为参考观察；这不代表你已观看原片。"),
    `## ${t("作品概述")}\n${result.summary}`,
    `## ${t("逐段声画")}`,
    ...result.segments.map((segment) =>
      [
        `### ${stamp(segment.start)} — ${stamp(segment.end)} · ${segment.title}`,
        ...(
          [
            ["画面", segment.visual],
            ["台词", segment.speech],
            ["画面文字", segment.onScreenText],
            ["声音", segment.sound],
            ["表达作用", segment.purpose],
          ] as const
        )
          .filter(([, value]) => value.trim())
          .map(([label, value]) => `${t(label)}：${value}`),
      ].join("\n\n"),
    ),
    `## ${t("创作方法")}`,
    ...Object.entries(craft).map(
      ([key, label]) =>
        `### ${t(label)}\n${result.craft[key as keyof typeof craft]}`,
    ),
    `## ${t("原片访问与局限")}\n${result.access}`,
  ]
    .filter(Boolean)
    .join("\n\n");
  return { text, agentId: "coordinator", refs: [], isolated: true };
}
export function analysisProject(state: AnalysisState) {
  const title = (state.work?.title || state.video?.name || t("参考视频"))
    .replace(/\.(mp4|mov|webm|m4v|mkv)$/i, "")
    .trim()
    .slice(0, 120);
  const project = newProject(t("{title} · 新创作", { title }));
  const { width = 0, height = 0 } = state.video ?? {};
  if (
    Number.isFinite(width) &&
    Number.isFinite(height) &&
    width > 0 &&
    height > 0
  ) {
    const scale = Math.min(1, 3840 / Math.max(width, height));
    project.width = Math.min(
      3840,
      Math.max(64, Math.round((width * scale) / 2) * 2),
    );
    project.height = Math.min(
      3840,
      Math.max(64, Math.round((height * scale) / 2) * 2),
    );
  }
  return project;
}
