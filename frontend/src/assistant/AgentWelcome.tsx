import { t, useLanguage } from "../i18n";
import { AgentMark } from "../ui/Identity";
import { ArrowUpRight, FilmStrip, ChatText } from "@phosphor-icons/react";
import { workflowRoles } from "./workflowAgents";
import type { WorkContext } from "./workContext";
export function AgentWelcome({
  hasContent,
  editingFrame = false,
  connected,
  suggest,
  connect,
  view,
}: {
  hasContent: boolean;
  editingFrame?: boolean;
  connected: boolean;
  suggest: (text: string, agentId: string) => void;
  connect: () => void;
  view?: WorkContext["view"];
}) {
  useLanguage();
  const prompts =
    view === "drafts"
      ? [
          {
            icon: ChatText,
            title: t("试生成图片"),
            agentId: "image",
            prompt: t(
              "请根据我选中的草稿和参考准备一个独立图片试生成任务，保留原稿，不关联正式镜头。",
            ),
          },
          {
            icon: FilmStrip,
            title: t("试生成视频"),
            agentId: "production",
            prompt: t(
              "请根据我选中的草稿和参考准备一个独立视频试生成任务，保留原稿，不关联正式镜头。",
            ),
          },
        ]
      : hasContent
        ? [
            {
              icon: ChatText,
              title: t("$编剧 · 打磨脚本"),
              agentId: "writer",
              prompt: t("请检查当前脚本的内容与声画表达，先给出具体修改建议。"),
            },
            {
              icon: FilmStrip,
              title: t("$分镜导演 · 设计镜头"),
              agentId: "director",
              prompt: t(
                "请根据已选脚本设计镜头，写清机位、调度、动作和切点，不改写脚本或生成媒体。",
              ),
            },
            {
              icon: FilmStrip,
              title: t("$图片制作 · 准备画面"),
              agentId: "image",
              prompt: t(
                "请检查当前镜头的静态画面需求、图片提示词与参考用途，指出需要补充的内容。",
              ),
            },
            {
              icon: FilmStrip,
              title: t("$视频制作 · 准备生成"),
              agentId: "production",
              prompt: t(
                "请检查当前分镜的视频生成提示词与参考素材用途，指出需要补充的内容。",
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
            {
              icon: ChatText,
              title: t("$创意策划 · 探索方向"),
              agentId: "concept",
              prompt: t(
                "请基于当前目标与素材比较几个不同创意方向，说明观看动机、核心事件和商品关系，不展开成完整脚本。",
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
              title: t("$编剧 · 撰写脚本"),
              agentId: "writer",
              prompt: t(
                "帮我写一支 30 秒短片的完整声画脚本，保存到脚本工作区。",
              ),
            },
            {
              icon: FilmStrip,
              title: t("$创意策划 · 探索方向"),
              agentId: "concept",
              prompt: t(
                "请先查看项目中的素材，基于已有内容提出可以实现的短片方案。",
              ),
            },
          ];
  const roles = workflowRoles(view);
  const starters = [
    ...prompts,
    {
      icon: ChatText,
      title: t("$资产 Agent · 建立镜头资产"),
      agentId: "assets",
      prompt: t(
        "请读取脚本与镜头需求，复用已确认资产，仅为缺失或指定修订的角色、场景和物品建立资产。来源素材继续保留；收录候选为素材不等于确认资产，候选须经确认后再供镜头使用。",
      ),
    },
  ].filter((p) => !roles || roles.includes(p.agentId));
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
            ? t(
                "素材供创意和脚本使用；资产供镜头制作使用。用 @ 引用当前对象，与对应 Agent 协作。",
              )
            : t(
                "用 $ 选择创意、编剧、导演、图片、视频或剪辑 Agent。不指定时由项目统筹梳理目标。",
              )}
      </p>
      <div className="agent-starters">
        {!editingFrame &&
          starters.map(({ title, prompt, agentId }) => (
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
