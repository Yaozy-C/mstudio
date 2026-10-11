import {
  assertGenerationReferences,
  replaceTaskReferences,
} from "./taskReferences";
import { normalizeTaskPrompt } from "./taskPrompt";
import { failure } from "../errors/failure";
import type { Project } from "../model";
import { saveTask } from "./document";
import { createTask, withMode } from "./tasks";
import type { ProductionTurn } from "./turnContext";
import type { ProductionTask } from "./types";

export type GenerationCommandContext = {
  callId: string;
  turnId: string;
  turn: ProductionTurn;
};
export const isRun = (t: ProductionTask) => !!t.turnId && !!t.createdAt;
export const runsOf = (p: Project, turnId?: string) =>
  Object.values(p.production?.drafts ?? {})
    .map(normalizeTaskPrompt)
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
  if (op.generationPurpose !== undefined && op.generationPurpose !== "asset")
    throw failure("VALIDATION_FAILED", "生成用途无效");
  const assetTask = op.generationPurpose === "asset";
  if (
    assetTask &&
    (op.mediaKind !== "image" ||
      op.id !== undefined ||
      op.canvasTaskKey !== undefined ||
      !Array.isArray(op.references))
  )
    throw failure(
      "VALIDATION_FAILED",
      "资产任务须为独立图片，并明确指定参考素材",
    );
  const kind = op.mediaKind ?? turn.task?.kind ?? "image";
  if (kind !== "image" && kind !== "video")
    throw failure("VALIDATION_FAILED", "生成类型无效");
  if (typeof op.text !== "string" || !op.text.trim() || op.text.length > 12000)
    throw failure("VALIDATION_FAILED", "请提供完整生成描述（最多 12000 字）");
  const prompt = op.text.trim();
  const ownerId = assetTask ? undefined : (op.id ?? turn.task?.ownerId);
  if (ownerId && !p.nodes.some((n) => n.id === ownerId && n.shot))
    throw failure("VALIDATION_FAILED", "目标镜头不存在");
  if (op.canvasTaskKey && op.canvasTaskKey !== turn.task?.key)
    throw failure("VALIDATION_FAILED", "请使用本轮消息引用的画布任务");
  const source =
    !assetTask && turn.task?.ownerId === ownerId ? turn.task : undefined;
  let task = source ? structuredClone(source) : createTask(p, [], kind);
  task = { ...task, kind };
  if (kind === "image" && op.mode !== undefined)
    throw failure("VALIDATION_FAILED", "图片任务不接受视频素材组合方式");
  const mode = op.mode ?? task.mode;
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
  // An explicit canvas task or reference list wins. Otherwise inherit the
  // shot's concrete media without replacing selected frame inputs.
  const shotReferences = p.nodes.find((n) => n.id === ownerId)?.references;
  if (
    op.references === undefined &&
    op.canvasTaskKey === undefined &&
    shotReferences?.length
  ) {
    op = {
      ...op,
      references: [
        ...task.inputs.filter((r) => r.role !== "script"),
        ...shotReferences.filter(
          (r) => !task.inputs.some((i) => i.assetId === r.assetId),
        ),
      ],
    };
  }
  task = replaceTaskReferences(p, task, op.references);
  assertGenerationReferences(p, task);
  const selectedModelId = turn.models[kind] || "";
  if (
    op.mediaModelId !== undefined &&
    (typeof op.mediaModelId !== "string" || !op.mediaModelId.trim())
  )
    throw failure("VALIDATION_FAILED", "媒体模型 ID 无效，请读取模型目录");
  if (selectedModelId && op.mediaModelId && op.mediaModelId !== selectedModelId)
    throw failure(
      "VALIDATION_FAILED",
      `请使用用户选择的模型 ${selectedModelId}；省略 mediaModelId 可沿用本轮选择。`,
    );
  // Explicit conversation selections are catalog-validated by the native tool.
  const modelId =
    selectedModelId || (op.mediaModelId as string | undefined) || "";
  const run: ProductionTask = {
    key,
    generationPurpose: assetTask ? "asset" : undefined,
    kind,
    mode: task.mode,
    inputs: task.inputs,
    ownerId: undefined,
    targetNodeId: typeof ownerId === "string" ? ownerId : undefined,
    prompt,
    parameters:
      op.parameters === undefined
        ? !assetTask && turn.task?.kind === kind
          ? turn.task.parameters
          : undefined
        : {
            ...(!assetTask && turn.task?.kind === kind
              ? turn.task.parameters
              : {}),
            ...(op.parameters as ProductionTask["parameters"]),
          },
    modelId,
    turnId,
    instruction: turn.instruction,
    createdAt: Date.now() + index,
    status:
      turn.models.execution === "automatic" && modelId
        ? "READY"
        : "AWAITING_CONFIRMATION",
  };
  if (runsOf(p, turnId).length >= 500)
    throw failure(
      "VALIDATION_FAILED",
      "本批次最多创建 500 个生成任务，请在新一轮继续",
    );
  return saveTask(p, run);
}
