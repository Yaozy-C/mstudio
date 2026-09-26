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
  memory: "整理项目记忆",
  delegate: "委派专业 Agent",
  plan: "安排交付清单",
};
const operations: Record<string, string> = {
  request_generation: "安排媒体生成",
  update_generation: "修改任务描述",
  regenerate_generation: "重新生成",
  add_node: "添加内容",
  update_node: "修改内容",
  remove_node: "删除内容",
  choose_take: "选用镜头",
  assemble_plan: "编排时间线",
  set_references: "整理参考素材",
};
export function activityRows(events: ActivityEvent[]) {
  type Payload = {
    callId?: string;
    name?: string;
    arguments?: { action?: string; operations?: { op: string }[] };
    result?: { error?: string; status?: string; generationTasks?: unknown[] };
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
          : (actions[action ?? ""] ?? "执行操作");
      return {
        id: e.seq,
        title,
        error: !!result?.result?.error,
        detail:
          result?.result?.error ??
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
