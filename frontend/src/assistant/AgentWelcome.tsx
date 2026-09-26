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
  const prompts = hasContent
    ? [
        {
          icon: ChatText,
          title: "$创意编剧 · 打磨故事",
          agentId: "concept",
          prompt: "请检查当前脚本的叙事和节奏，先给出修改建议。",
        },
        {
          icon: FilmStrip,
          title: "$分镜导演 · 细化镜头",
          agentId: "storyboard",
          prompt: "请查看当前分镜，补充每个镜头的机位、运动与动作节拍。",
        },
        {
          icon: ChatText,
          title: "$分镜画手 · 准备画面",
          agentId: "storyboard-artist",
          prompt: "请检查分镜构图、画格状态与图片参考，指出需要补充的内容。",
        },
      ]
    : [
        {
          icon: ChatText,
          title: "$项目统筹 · 梳理目标",
          agentId: "coordinator",
          prompt: "我想做一支短片，请先帮我梳理主题、受众和视觉方向。",
        },
        {
          icon: ChatText,
          title: "$创意编剧 · 设计故事",
          agentId: "concept",
          prompt:
            "帮我设计一支 30 秒短片的创意方向和完整故事，保存为视频方案。",
        },
        {
          icon: FilmStrip,
          title: "$创意编剧 · 围绕素材",
          agentId: "concept",
          prompt: "请先查看项目中的素材，基于已有内容提出可以实现的短片方案。",
        },
      ];
  return (
    <div className="agent-welcome">
      <div className="agent-welcome-mark">
        <AgentMark size={28} />
      </div>
      <div className="eyebrow">LET’S CREATE TOGETHER</div>
      <h3>
        {editingFrame
          ? "想怎么调整？"
          : hasContent
            ? "接下来，一起打磨。"
            : "你的下一个故事，从这里开始。"}
      </h3>
      <p>
        {editingFrame
          ? "选中的素材已带入。直接说想怎么改，Agent 会在对话里制作，结果同时放回画布。"
          : hasContent
            ? "输入 @ 选择脚本、镜头或素材，告诉我想改什么。"
            : "用 $ 选择创意、分镜、制作、剪辑或审片 Agent。不指定时由项目统筹梳理目标。"}
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
          连接对话模型
          <ArrowUpRight />
        </button>
      )}
      <small>
        {editingFrame
          ? "点结果，接着聊、接着改。图片和视频模型在输入框下方选择。"
          : "修改写入当前项目 · 生成内容前由你确认"}
      </small>
    </div>
  );
}
