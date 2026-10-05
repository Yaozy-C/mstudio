import { normalizeTaskPrompt } from "./taskPrompt";
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
  if (
    task.inputs.some((r) =>
      p.assets.some((a) => a.id === r.assetId && a.inLibrary === false),
    )
  )
    throw failure(
      "VALIDATION_FAILED",
      "参考素材已从素材库移除，请移除该参考或先恢复素材",
    );
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
