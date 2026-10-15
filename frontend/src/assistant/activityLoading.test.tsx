import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentActivityFeed } from "./AgentActivityFeed";
import type { ActivityEvent } from "./activityRows";
import type { ChildActivity } from "./childActivity";

const call: ActivityEvent = {
  seq: 1,
  created: 10,
  kind: "tool/call",
  payload: {
    callId: "delegate",
    name: "mstudio_delegate",
    arguments: { agentId: "director" },
  },
};
const receipt: ActivityEvent = {
  seq: 2,
  created: 11,
  kind: "tool/result",
  payload: { callId: "delegate", result: { status: "running" } },
};
const child: ChildActivity = {
  projectId: "p",
  parentTurn: "parent",
  turnId: "child-turn",
  childId: "child",
  callId: "delegate",
  agentId: "director",
  state: "running",
  phase: { kind: "request/start" },
  note: "核对参考画面",
  started: 10,
  updated: 20,
  events: [],
};

function render(events: ActivityEvent[], children: ChildActivity[] = []) {
  return renderToStaticMarkup(
    <AgentActivityFeed
      events={events}
      children={children}
      seconds={10}
      running
      status="正在执行工具…"
    />,
  );
}

test("a running tool or delegation owns the loading indicator without a duplicate footer", () => {
  for (const html of [
    render([
      { ...call, payload: { callId: "read", name: "mstudio_read_image" } },
    ]),
    render([call]),
    render([call, receipt]),
    render([call, receipt], [child]),
  ]) {
    expect(html.match(/class="status-spinner"/g)).toHaveLength(1);
    expect(html).not.toContain('class="agent-activity-status"');
  }
});

test("the parent loading indicator returns once the live child settles", () => {
  for (const state of [
    "completed",
    "cancelled",
    "failed",
    "interrupted",
  ] as const) {
    const html = render([call, receipt], [{ ...child, state }]);
    expect(html.match(/class="status-spinner"/g)).toHaveLength(1);
    expect(html).toContain('class="agent-activity-status"');
    expect(html).not.toContain("子任务执行中");
  }
});

test("the footer still shows loading when no visible tool row owns it", () => {
  for (const html of [
    render([]),
    render([], [child]),
    render([
      call,
      { ...receipt, payload: { callId: "delegate", result: { ok: true } } },
    ]),
  ]) {
    expect(html.match(/class="status-spinner"/g)).toHaveLength(1);
    expect(html).toContain('class="agent-activity-status"');
  }
});
