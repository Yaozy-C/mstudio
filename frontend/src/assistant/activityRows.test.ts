import { expect, test } from "bun:test";
import { activityRows } from "./activityRows";
test("new named project and memory tools share call/result status", () => {
  const rows = activityRows([
    {
      seq: 1,
      created: 1,
      kind: "tool/call",
      payload: {
        callId: "a",
        name: "mstudio_edit",
        arguments: { operations: [{ op: "add_node" }] },
      },
    },
    {
      seq: 2,
      created: 2,
      kind: "tool/result",
      payload: { callId: "a", result: { applied: true } },
    },
    {
      seq: 3,
      created: 3,
      kind: "tool/call",
      payload: {
        callId: "b",
        name: "mstudio_memory",
        arguments: { action: "remember" },
      },
    },
    {
      seq: 4,
      created: 4,
      kind: "tool/result",
      payload: {
        callId: "b",
        result: { error: "已停止", code: "ABORTED_BEFORE_DISPATCH" },
      },
    },
  ]);
  expect(rows.map((r) => [r.title, r.detail])).toEqual([
    ["添加内容", "已完成"],
    ["整理项目记忆", "已停止"],
  ]);
  expect(rows[1].error).toBe(true);
});

test("delegation return and media task creation do not claim generation completion", () => {
  for (const [result, expected] of [
    [{ status: "running" }, "子任务执行中"],
    [{ status: "completed" }, "专业 Agent 已返回"],
    [{ error: "生成任务未创建" }, "生成任务未创建"],
  ] as const) {
    const rows = activityRows([
      {
        seq: 1,
        created: 1,
        kind: "tool/call",
        payload: { callId: "a", name: "mstudio_delegate" },
      },
      {
        seq: 2,
        created: 2,
        kind: "tool/result",
        payload: { callId: "a", result },
      },
    ]);
    expect(rows[0].detail).toBe(expected);
  }
});
