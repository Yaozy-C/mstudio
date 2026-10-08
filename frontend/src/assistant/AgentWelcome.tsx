import { t, useLanguage } from "../i18n";
import { AgentMark } from "../ui/Identity";
import { ArrowUpRight, FilmStrip, ChatText } from "@phosphor-icons/react";
export function AgentWelcome({
  hasContent,
  editingFrame = false,
  connected,
  suggest,
  connect,
}: {
  hasContent: boolean;
  editingFrame?: boolean;
  connected: boolean;
  suggest: (text: string, agentId: string) => void;
  connect: () => void;
}) {
  useLanguage();
  const prompts = hasContent
    ? [
        {
          icon: ChatText,
          title: t("$创意与导演 · 打磨内容"),
          agentId: "concept",
          prompt: t(
            "请检查当前脚本或分镜的内容、表演与节奏，先给出具体修改建议。",
          ),
        },
        {
          icon: FilmStrip,
          title: t("$媒体制作 · 准备生成"),
          agentId: "production",
          prompt: t(
            "请检查当前分镜的画面、参考素材用途与生成提示词，指出需要补充的内容。",
          ),
        },
        {
          icon: ChatText,
          title: t("$剪辑与后期 · 整理成片"),
          agentId: "editor",
          prompt: t(
            "请查看当前时间线，说明可用的素材区间、节奏问题以及调色或转场建议。",
          ),
        },
      ]
    : [
        {
          icon: ChatText,
          title: t("$项目统筹 · 梳理目标"),
          agentId: "coordinator",
          prompt: t("我想做一支短片，请先帮我梳理主题、受众和视觉方向。"),
        },
        {
          icon: ChatText,
          title: t("$创意与导演 · 设计故事"),
          agentId: "concept",
          prompt: t("帮我写一支 30 秒短片的完整声画脚本，保存到脚本工作区。"),
        },
        {
          icon: FilmStrip,
          title: t("$创意与导演 · 围绕素材"),
          agentId: "concept",
          prompt: t(
            "请先查看项目中的素材，基于已有内容提出可以实现的短片方案。",
          ),
        },
      ];
  return (
    <div className="agent-welcome">
      <div className="agent-welcome-mark">
        <AgentMark size={28} />
      </div>
      {editingFrame && <h3>{t("想怎么调整？")}</h3>}
      <p>
        {editingFrame
          ? t(
              "选中的素材已带入。直接说想怎么改，Agent 会在对话里制作，结果同时放回画布。",
            )
          : hasContent
            ? t("输入 @ 选择脚本、镜头或素材，告诉我想改什么。")
            : t(
                "用 $ 选择创意、分镜、制作、剪辑或审片 Agent。不指定时由项目统筹梳理目标。",
              )}
      </p>
      <div className="agent-starters">
        {!editingFrame &&
          prompts.map(({ title, prompt, agentId }) => (
            <button key={title} onClick={() => suggest(prompt, agentId)}>
              <AgentMark id={agentId} size={20} />
              <span>{title}</span>
              <ArrowUpRight />
            </button>
          ))}
      </div>
      {!connected && (
        <button className="agent-connect-callout" onClick={connect}>
          {t("连接对话模型")}
          <ArrowUpRight />
        </button>
      )}
    </div>
  );
}
