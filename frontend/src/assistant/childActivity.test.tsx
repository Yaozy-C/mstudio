import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ChildAgentActivity } from "./ChildAgentActivity";
import {
  belongsTo,
  childStatus,
  elapsed,
  type ChildActivity,
} from "./childActivity";
const child: ChildActivity = {
  projectId: "p",
  parentTurn: "parent",
  turnId: "child-turn",
  childId: "child",
  callId: "delegate",
  agentId: "art",
  state: "running",
  phase: { kind: "tool/call", payload: { name: "mstudio_read_image" } },
  note: "核对人物参考和光线",
  started: 10,
  updated: 20,
  events: [
    {
      seq: 1,
      created: 10,
      kind: "tool/call",
      payload: { callId: "read", name: "mstudio_read_image" },
    },
  ],
};
test("child progress displays the actual tool, note and duration outside collapsed history", () => {
  expect(childStatus(child)).toBe("读取素材画面");
  expect(elapsed(child, 280)).toBe(270);
  const html = renderToStaticMarkup(
    <ChildAgentActivity child={child} name="分镜画手" />,
  );
  expect(html).toContain("分镜画手");
  expect(html).toContain("读取素材画面");
  expect(html).toContain("核对人物参考和光线");
  expect(html).not.toContain("<details");
  expect(
    renderToStaticMarkup(
      <ChildAgentActivity child={child} name="分镜画手" showTools />,
    ),
  ).toContain("<details");
});
test("model wait, backend wait, cancellation and restart have distinct states", () => {
  expect(childStatus({ ...child, phase: { kind: "request/start" } })).toBe(
    "正在等待模型响应…",
  );
  expect(
    childStatus({
      ...child,
      phase: {
        kind: "tool/call",
        payload: { name: "mstudio_await_generation" },
      },
    }),
  ).toBe("等待生成结果");
  expect(childStatus({ ...child, state: "cancelled" })).toBe("已停止");
  expect(childStatus({ ...child, state: "interrupted" })).toBe("执行已中断");
  expect(elapsed({ ...child, state: "completed" }, 280)).toBe(10);
  expect(belongsTo(child, "p", "parent")).toBe(true);
  expect(belongsTo(child, "other", "parent")).toBe(false);
  expect(belongsTo(child, "p", "other")).toBe(false);
});
