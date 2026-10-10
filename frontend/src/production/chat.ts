import type { Project } from "../model";
import {
  describeAttachment,
  type Attachment,
  type AttachmentRef,
} from "../assistant/attachments";
import type { GenerationPreferences, ProductionTask } from "./types";
export function taskAttachments(
  p: Project,
  task?: ProductionTask,
  extra: AttachmentRef[] = [],
): Attachment[] {
  const refs: AttachmentRef[] = task
    ? task.inputs.flatMap((r): AttachmentRef[] =>
        r.sourceRef
          ? [r.sourceRef]
          : r.assetId
            ? [{ kind: "asset", id: r.assetId }]
            : r.nodeId
              ? [{ kind: "node", id: r.nodeId }]
              : [],
      )
    : [];
  refs.push(...extra);
  const map = new Map(
    refs.map((r) => [`${r.kind}:${r.id}`, describeAttachment(p, r)]),
  );
  return [...map.values()].filter((r): r is Attachment => !!r);
}
export function taskContext(
  task?: ProductionTask,
  models?: GenerationPreferences,
  referencedTask?: ProductionTask,
) {
  return `\n\n本轮制作上下文（参考数据）：${JSON.stringify({
    referencedTask,
    canvasTaskKey: task?.key,
    ownerId: task?.ownerId,
    kind: task?.kind,
    inputs: task?.inputs,
    preparedPrompt: task?.prompt,
    selectedModels: models,
  })}\n用户要求生成或修改图片/视频时，使用对应的 mstudio_generate_image、mstudio_generate_video 或 mstudio_generate_reference_image 工具创建生成任务，prompt 为完整生成描述；按工具字段指定 mode 和 references 的用途，应用负责执行并显示进度与结果。统筹交接目标与用户要求，由对应媒体角色根据所选模型落实完整描述、参考用途与实际参数；任务设置可调整参数，但不能代替落实用户明确规格。工具或模型不支持时说明具体缺口，不静默按默认规格提交；常规实现选择自行处理，不反复索要确认。任务详情在任务栏，聊天保留入口，结果同时放到画布。按镜头要求“生成／重新生成分镜图”时，按当前动作分镜板规则每镜头交付一张整板，不因历史上存在多张单帧任务就逐条重试；旧图片与历史记录保留。只有用户明确指定某个旧任务原样重试时，才沿用它的旧格式。引用已有任务时用 inspect section=generation、taskKey 读取最新记录；用 mstudio_update_generation(taskKey,prompt) 保存 prompt，统一更新该任务的 prompt。仅改 prompt 不生成；用户明确要求改完重新生成时，先保存，再 mstudio_regenerate_generation(taskKey) 创建新任务，沿用该任务的模型、参考和参数。隐藏记录不影响查询，不代表取消，不因隐藏或停止自动重提。模型使用用户选择，缺失时任务卡让用户选择。只讨论或写 Prompt 时不发起生成。`;
}
