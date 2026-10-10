import { expect, test } from "bun:test";
import { activityRows } from "./activityRows";
test("named project tools share call/result status", () => {
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
        name: "mstudio_update_shot_design",
        arguments: {},
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
    ["修改镜头设计", "已停止"],
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

test("a limited child run is shown as recovered only when its parent completes", () => {
  const events = [
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
      payload: {
        callId: "a",
        result: { error: "limit", stopReason: "step-limit" },
      },
    },
  ];
  expect(activityRows(events)[0].error).toBe(false);
  expect(activityRows(events)[0].detail).toContain("待统筹处理");
  expect(
    activityRows([
      ...events,
      {
        seq: 3,
        created: 3,
        kind: "turn/end",
        payload: { status: "completed" },
      },
    ])[0].detail,
  ).toContain("已完成本轮任务");
  expect(
    activityRows([
      ...events,
      { seq: 3, created: 3, kind: "turn/end", payload: { status: "failed" } },
    ])[0].detail,
  ).toContain("待统筹处理");
});

test("nested validation errors show the cause and recovery instead of raw JSON", () => {
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
      payload: {
        callId: "a",
        result: {
          error:
            '连续三次修改遇到相同错误：Error: {"code":"VALIDATION_FAILED","message":"请选择正确模型","recovery":"修改后重试","retryable":false}',
        },
      },
    },
  ]);
  expect(rows[0].error).toBe(true);
  expect(rows[0].detail).toContain("连续三次");
  expect(rows[0].detail).toContain("请选择正确模型");
  expect(rows[0].detail).not.toContain('"code"');
});
