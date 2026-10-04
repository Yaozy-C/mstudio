import { normalizeError, errorCatalog, redactDetails } from "../errors/catalog";
export type ActivityEvent = {
  seq: number;
  kind: string;
  created: number;
  payload: unknown;
};
const actions: Record<string, string> = {
  inspect: "读取项目内容",
  edit: "更新项目",
  skills: "查找创作能力",
  read_skill: "读取创作规则",
  history: "查阅对话",
  models: "检查可用模型",
  read_image: "读取素材画面",
  reopen_image: "重新读取素材画面",
  read_result: "读取完整结果",
  await_generation: "等待生成结果",
  memory: "整理项目记忆",
  delegate: "委派专业 Agent",
  plan: "安排交付清单",
};
const operations: Record<string, string> = {
  generate_image: "生成图片",
  generate_video: "生成视频",
  generate_reference_image: "生成参考图片",
  set_video_prompt: "修改视频提示词",
  set_image_prompt: "修改图片提示词",
  set_shot_frames: "设置分镜图片",
  update_shot: "修改镜头",
  update_shots: "批量修改镜头",
  update_screenplay: "修改剧本",
  upsert_references: "添加或更新参考素材",
  replace_references: "替换参考素材",
  remove_references: "移除参考素材",
  request_generation: "安排媒体生成",
  update_generation: "修改任务描述",
  regenerate_generation: "重新生成",
  add_node: "添加内容",
  add_shot: "添加镜头",
  add_screenplay: "添加剧本",
  add_asset: "添加素材",
  add_text: "添加文本",
  add_note: "添加备注",
  update_node: "修改内容",
  remove_node: "删除内容",
  choose_take: "选用镜头",
  assemble_screenplay: "编排时间线",
  set_references: "整理参考素材",
};
export function activityRows(events: ActivityEvent[]) {
  type Payload = {
    callId?: string;
    name?: string;
    arguments?: {
      action?: string;
      agentId?: string;
      operations?: { op: string }[];
    };
    result?: {
      agentId?: string;
      error?: string;
      status?: string;
      stopReason?: string;
      generationTasks?: unknown[];
      issues?: { path: string; message: string }[];
    };
  };
  return events
    .filter((e) => e.kind === "tool/call")
    .sort((a, b) => a.seq - b.seq)
    .map((e) => {
      const p = e.payload as Payload;
      const result = events.find(
        (v) =>
          v.kind === "tool/result" &&
          (v.payload as Payload)?.callId === p.callId,
      )?.payload as Payload | undefined;
      const action =
        p.name === "mstudio_memory"
          ? "memory"
          : (p.arguments?.action ?? p.name?.replace(/^mstudio_/, ""));
      const title =
        action === "edit"
          ? [
              ...new Set(
                p.arguments?.operations?.map(
                  (o) => operations[o.op] ?? "修改项目",
                ) ?? [],
              ),
            ].join("、") || "修改项目"
          : (actions[action ?? ""] ?? operations[action ?? ""] ?? "执行操作");
      const limited =
        action === "delegate" && result?.result?.stopReason === "step-limit";
      const completed = events.some(
        (v) =>
          v.kind === "turn/end" &&
          (v.payload as { status?: string }).status === "completed",
      );
      return {
        id: e.seq,
        callId: p.callId,
        title,
        agentId:
          action === "delegate"
            ? p.arguments?.agentId || result?.result?.agentId
            : undefined,
        error: !!result?.result?.error && !limited,
        detail:
          (limited
            ? completed
              ? "此次委派达到步数上限；统筹后续已完成本轮任务"
              : "专业 Agent 达到步数上限，已完成的修改保留，待统筹处理"
            : undefined) ??
          (result?.result?.error
            ? [
                activityError(result.result.error),
                ...(result.result.issues ?? []).map(
                  (issue) => `${issue.path}: ${issue.message}`,
                ),
              ].join("\n")
            : undefined) ??
          (!result
            ? "执行中"
            : result.result?.status === "running"
              ? "子任务执行中"
              : action === "delegate"
                ? "专业 Agent 已返回"
                : result.result?.generationTasks?.length
                  ? "已创建生成任务"
                  : "已完成"),
      };
    });
}

function activityError(raw: string): string {
  const start = raw.indexOf("{");
  if (start < 0) return redactDetails(raw);
  try {
    const data = JSON.parse(raw.slice(start));
    if (!data || !Object.hasOwn(errorCatalog, data.code))
      return redactDetails(raw);
    const issue = normalizeError(data);
    const prefix = raw
      .slice(0, start)
      .replace(/Error:\s*$/, "")
      .trim();
    return [prefix, issue.message, issue.recovery].filter(Boolean).join("\n");
  } catch {
    return redactDetails(raw);
  }
}
