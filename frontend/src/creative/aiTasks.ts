import type { AttachmentRef } from "../assistant/attachments";
import { flushPendingEdits } from "../workspace/pendingEdits";
export type CreativeTask = {
  text: string;
  agentId?: string;
  refs?: AttachmentRef[];
  targetNodeId?: string;
};
export function creativeTask(
  kind: "write" | "revise-script" | "split" | "revise-shot",
  id?: string,
  paragraph?: string,
  idea = "",
): CreativeTask {
  const target = id
    ? "请查看本次引用的内容。"
    : "请先查看当前项目目标和已有素材。";
  const scope = paragraph ? `只处理${paragraph}。` : "";
  const prompts = {
    write: `${id ? "起草这份声画脚本" : "为项目起草脚本，已有脚本时更新原稿，仅没有脚本时创建"}。${target}先根据我的想法和本轮实际附带的资料梳理内容；素材库目录不代表你已读到其内容，缺少必要资料时请我用 @ 补充；关键信息不足时追问，不编造商品事实。把脚本保存到脚本工作区，按段落组织标题、可见动作、onScreenText 画面文字、台词、关键声音和 duration 预估秒数；区分时长上限、锁定值与暂估，不凑满。本轮只写脚本，等待我确认，不拆镜头、不生成图片或视频。`,
    "revise-script": `请协助修改这份脚本。${target}${scope}修改要求不明确时再询问；已有明确要求就直接修改对应段落，保留段落关联，保留其他已确认内容和关联镜头。不要自动生成图片或视频。`,
    split: `请根据完整脚本设计镜头。${target}${scope}先通读全片脚本、已有镜头与约束，决定观看顺序、揭晓、必须连续呈现的动作、可省略过程和切点，再保存本次范围内的镜头。局部任务也参考前后衔接，但不改无关镜头。时长依据动作与辨认需要，暂估段落时间可重新分配并说明，遵守用户明确锁定或上限。不机械逐段均分，一段可一镜或多镜。写清镜头意图、动作、摄影、画面文字和声音落点、台词与时长，关联来源段落。已有镜头沿用 ID 调整，避免重复创建；保留脚本与已有素材。本轮只完成文字镜头设计，不生成图片或视频。`,
    "revise-shot": `请协助修改这个镜头。${target}读取来源脚本和当前镜头，先问我想改动作、景别、构图还是画面。确认要求后只改当前镜头的文字方案，保留其他镜头和已有图片、视频。若我要求重做分镜图，交给媒体制作 Agent 在对话中创建生成任务。`,
  };
  return {
    agentId: "concept",
    targetNodeId: id,
    refs: [],
    text: prompts[kind] + (idea.trim() ? `\n\n我的想法：${idea.trim()}` : ""),
  };
}
export function requestCreativeTask(task: CreativeTask) {
  flushPendingEdits();
  window.dispatchEvent(
    new CustomEvent("studio-creative-request", { detail: task }),
  );
}
