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
  })}\n用户要求生成或修改图片/视频时，使用 request_generation 创建生成任务，text 为完整生成描述；按要求指定 mediaKind、mode 和 references 的用途，应用负责执行并显示进度与结果。统筹交接目标与用户要求，由对应媒体角色根据所选模型落实完整描述、参考用途与实际参数；任务设置可调整参数，但不能代替落实用户明确规格。工具或模型不支持时说明具体缺口，不静默按默认规格提交；常规实现选择自行处理，不反复索要确认。任务详情在任务栏，聊天保留入口，结果同时放到画布。引用已有任务时用 inspect section=generation、taskKey 读取最新记录；用 update_generation(taskKey,text) 保存 prompt，统一更新该任务的 prompt。仅改 prompt 不生成；用户明确要求改完重新生成时，先保存，再 regenerate_generation(taskKey) 创建新任务，沿用该任务的模型、参考和参数。隐藏记录不影响查询，不代表取消，不因隐藏或停止自动重提。仅使用本轮明确指定的生成素材；读取分镜或素材目录不代表将它们全部作为生成输入。模型使用用户选择，缺失时任务卡让用户选择。只讨论或写 Prompt 时不发起生成。`;
}
