import type { AttachmentRef } from "../assistant/attachments";
import type { CreativeTask } from "../creative/aiTasks";
import type { EffectPreset } from "./effects";

export function effectAgentTask(
  effect: EffectPreset,
  ref?: AttachmentRef,
): CreativeTask {
  return {
    agentId: "production",
    refs: ref ? [ref] : [],
    text: [
      `我想使用「${effect.name}」特效。请结合本轮要求、实际引用的素材、当前镜头与项目上下文，融合下面的效果 Prompt，再交给合适的视频模型生成。`,
      `效果 Prompt：\n${effect.prompt}`,
      `适配要点：${effect.adaptation}`,
      "先读取实际引用的内容，素材目录不等于已看过素材。将 [主体]、[材质]、[方向]、[范围]、[节奏] 等占位符替换为当前任务的具体内容；没有明确对象时询问应用到哪个镜头或素材，不强制上传图片。",
      "保留用户已确认的叙事、主体身份、场景和品牌要求。处理镜头运动、原有动作、特效变化及时间限制之间的冲突，不机械拼接，不添加示例中的商品或场景。涉及形状或材质变化时，明确允许变化的范围，避免同时要求该范围保持原样。",
      "根据实际配置的视频模型与输入能力，选择文生视频、图生视频或视频编辑；视频参考不等于视频编辑，不把取帧重生成说成保留原片动作。将画面动作、镜头和变化顺序写成模型可执行的最终 Prompt，按模型需要简化并使用正向描述。",
      "只追问阻碍执行的关键信息。目标明确且生成已获授权时，遵循项目现有的模型配置与生成确认流程提交；缺少必要输入或可用模型时说明原因。保留原素材，将结果作为新版本供预览，不宣称未完成的生成成功。",
    ].join("\n\n"),
  };
}
