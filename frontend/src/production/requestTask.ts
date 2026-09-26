import { failure } from "../errors/failure";
import type { Project } from "../model";
import { attachmentInput } from "./attachmentInput";
import { saveTask } from "./document";
import { productionItems } from "./items";
import { createTask, withMode } from "./tasks";
import type { ProductionTurn } from "./turnContext";
import type { ProductionInput, ProductionTask } from "./types";

export type GenerationCommandContext = {
  callId: string;
  turnId: string;
  turn: ProductionTurn;
};
export const isRun = (t: ProductionTask) => !!t.turnId && !!t.createdAt;
export const runsOf = (p: Project, turnId?: string) =>
  Object.values(p.production?.drafts ?? {})
    .filter((t) => isRun(t) && (!turnId || t.turnId === turnId))
    .sort((a, b) => a.createdAt! - b.createdAt!);

export function requestTask(
  p: Project,
  op: Record<string, unknown>,
  context?: GenerationCommandContext,
  index = 0,
): Project {
  if (!context || context.turn.projectId !== p.id)
    throw failure("VALIDATION_FAILED", "本轮生成上下文已结束，请重新发送要求");
  const { turn, turnId, callId } = context;
  const key = `run:${callId}:${index}`;
  if (p.production?.drafts?.[key]) return p;
  const kind = op.mediaKind ?? turn.task?.kind ?? "image";
  if (kind !== "image" && kind !== "video")
    throw failure("VALIDATION_FAILED", "生成类型无效");
  if (typeof op.text !== "string" || !op.text.trim() || op.text.length > 12000)
    throw failure("VALIDATION_FAILED", "请提供完整生成描述（最多 12000 字）");
  const ownerId = op.id ?? turn.task?.ownerId;
  if (ownerId && !p.nodes.some((n) => n.id === ownerId && n.shot))
    throw failure("VALIDATION_FAILED", "目标镜头不存在");
  if (op.canvasTaskKey && op.canvasTaskKey !== turn.task?.key)
    throw failure("VALIDATION_FAILED", "请使用本轮消息引用的画布任务");
  const source = turn.task?.ownerId === ownerId ? turn.task : undefined;
  let task = source ? structuredClone(source) : createTask(p, [], kind);
  task = { ...task, kind };
  const mode = op.mode ?? (kind === "image" ? "multi" : task.mode);
  if (!["single", "ends", "multi", "mixed"].includes(String(mode)))
    throw failure("VALIDATION_FAILED", "素材组合方式无效");
  task =
    kind === "video"
      ? withMode(task, mode as ProductionTask["mode"])
      : {
          ...task,
          mode: "multi",
          inputs: task.inputs.map((r) => ({
            ...r,
            role:
              r.role === "first-frame" || r.role === "last-frame"
                ? "reference"
                : r.role,
          })),
        };
  if (op.references !== undefined) {
    if (!Array.isArray(op.references) || op.references.length > 12)
      throw failure("VALIDATION_FAILED", "最多使用 12 个参考素材");
    const inputs = op.references.map((value): ProductionInput => {
      if (!value || typeof value !== "object")
        throw failure("VALIDATION_FAILED", "参考素材无效");
      const r = value as Record<string, unknown>;
      if (typeof r.assetId !== "string")
        throw failure("VALIDATION_FAILED", "缺少参考素材 ID");
      const input = attachmentInput(p, { kind: "asset", id: r.assetId });
      const asset = p.assets.find((a) => a.id === r.assetId);
      if (!input || !asset || !["image", "video"].includes(asset.kind))
        throw failure("VALIDATION_FAILED", "只能使用项目中已有的图片或视频");
      const role =
        r.role ?? (asset.kind === "video" ? "video-reference" : "reference");
      const allowed =
        asset.kind === "image"
          ? kind === "image"
            ? ["edit", "reference"]
            : ["reference", "first-frame", "last-frame"]
          : ["video-reference"];
      if (!allowed.includes(String(role)))
        throw failure("VALIDATION_FAILED", "素材用途与类型不匹配");
      const frame = productionItems(p).find(
        (n) => n.kind === "image" && n.assetId === asset.id,
      );
      for (const field of ["start", "end"])
        if (
          r[field] !== undefined &&
          (typeof r[field] !== "number" || !Number.isFinite(r[field]))
        )
          throw failure("VALIDATION_FAILED", "视频区间必须是有效秒数");
      const start = (r.start as number | undefined) ?? input.start;
      const end = (r.end as number | undefined) ?? input.end;
      if (
        asset.kind === "video" &&
        (start === undefined ||
          end === undefined ||
          start < 0 ||
          end > asset.duration ||
          end <= start)
      )
        throw failure("VALIDATION_FAILED", "请指定有效的视频参考区间");
      return {
        ...input,
        role: role as ProductionInput["role"],
        start,
        end,
        nodeId: frame?.nodeId ?? input.nodeId,
        frame: !!frame && !!p.nodes.find((n) => n.id === frame.nodeId)?.shot,
        purpose:
          typeof r.purpose === "string"
            ? r.purpose.slice(0, 1000)
            : input.purpose,
      };
    });
    if (new Set(inputs.map((r) => r.assetId)).size !== inputs.length)
      throw failure("VALIDATION_FAILED", "参考素材不能重复");
    task.inputs = [
      ...task.inputs.filter((r) => r.role === "script"),
      ...inputs,
    ];
  }
  const modelId = turn.models[kind] || "";
  if (op.mediaModelId && op.mediaModelId !== modelId)
    throw failure(
      "VALIDATION_FAILED",
      "请使用用户选择的模型；未选择时创建待选模型的任务卡",
    );
  const run: ProductionTask = {
    key,
    kind,
    mode: task.mode,
    inputs: task.inputs,
    ownerId: undefined,
    prompt: op.text.trim(),
    parameters: turn.task?.kind === kind ? turn.task.parameters : undefined,
    modelId,
    turnId,
    instruction: turn.instruction,
    createdAt: Date.now() + index,
    status:
      turn.models.execution === "automatic" && modelId
        ? "READY"
        : "AWAITING_CONFIRMATION",
  };
  // A repeated tool call in this turn must not silently create a second paid job.
  if (
    runsOf(p, turnId).some(
      (t) =>
        t.kind === run.kind &&
        t.ownerId === run.ownerId &&
        t.prompt === run.prompt &&
        JSON.stringify(t.inputs) === JSON.stringify(run.inputs),
    )
  )
    return p;
  if (runsOf(p, turnId).length >= 12)
    throw failure(
      "VALIDATION_FAILED",
      "本轮最多创建 12 个生成任务，请分批制作",
    );
  return saveTask(p, run);
}
