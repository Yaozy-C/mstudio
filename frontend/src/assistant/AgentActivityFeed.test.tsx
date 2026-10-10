import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentActivityFeed } from "./AgentActivityFeed";
import type { ChildActivity } from "./childActivity";

const events = [
  {
    seq: 1,
    created: 1,
    kind: "assistant/reasoning",
    payload: { text: "先核对参考" },
  },
  {
    seq: 2,
    created: 2,
    kind: "tool/call",
    payload: {
      callId: "read",
      name: "mstudio_read_image",
      arguments: { assetId: "image-1" },
    },
  },
  {
    seq: 3,
    created: 3,
    kind: "tool/result",
    payload: { callId: "read", result: { ok: true } },
  },
  {
    seq: 4,
    created: 4,
    kind: "assistant/reasoning",
    payload: { text: "再修改镜头 <script>" },
  },
];

test("thinking and tools are independently collapsed while the whole timeline stays visible", () => {
  for (const running of [true, false]) {
    const html = renderToStaticMarkup(
      <AgentActivityFeed events={events} seconds={65} running={running} />,
    );
    expect(html).toContain("已处理 1分 5秒");
    expect(html).not.toContain("agent-activity-process");
    expect(html.match(/<details /g)).toHaveLength(3);
    expect(html).not.toMatch(/<details[^>]*\bopen(?:=|[ >])/);
    expect(html.indexOf("先核对参考")).toBeLessThan(
      html.indexOf("读取素材画面"),
    );
    expect(html.indexOf("读取素材画面")).toBeLessThan(
      html.indexOf("再修改镜头"),
    );
    expect(html).toContain("&lt;script&gt;");
    expect(html).toContain("image-1");
    expect(html).toContain("返回结果");
    expect(html.includes('role="status"')).toBe(running);
  }
});

test("child task is a sibling of the delegate operation, with its own thinking and tools", () => {
  const child: ChildActivity = {
    projectId: "p",
    parentTurn: "parent",
    turnId: "child-turn",
    childId: "child",
    callId: "delegate",
    agentId: "image",
    state: "completed",
    phase: {},
    note: "",
    started: 10,
    updated: 20,
    events: events.map((event) =>
      event.seq === 1 ? { ...event, payload: { text: "子任务的思考" } } : event,
    ),
  };
  const html = renderToStaticMarkup(
    <AgentActivityFeed
      seconds={10}
      running={false}
      children={[child]}
      agents={[{ id: "image", name: "图像制作" }]}
      events={[
        {
          seq: 1,
          created: 1,
          kind: "tool/call",
          payload: {
            callId: "delegate",
            name: "mstudio_delegate",
            arguments: { agentId: "image" },
          },
        },
        {
          seq: 2,
          created: 2,
          kind: "tool/result",
          payload: { callId: "delegate", result: { status: "completed" } },
        },
      ]}
    />,
  );
  const task = html.indexOf('class="agent-child-activity"');
  expect(task).toBeGreaterThan(html.indexOf("</details>"));
  // No disclosure remains open around the independent task card.
  const prefix = html.slice(0, task);
  expect(prefix.match(/<details/g)?.length).toBe(
    prefix.match(/<\/details>/g)?.length,
  );
  expect(html).toContain("子任务的思考");
  expect(html.match(/class="agent-child-activity"/g)).toHaveLength(1);
  expect(html).toContain("图像制作");
});

test("child remains visible when its delegation event is outside retained history", () => {
  const child: ChildActivity = {
    projectId: "p",
    parentTurn: "parent",
    turnId: "child-turn",
    childId: "child",
    callId: "old-call",
    agentId: "image",
    state: "failed",
    phase: {},
    note: "",
    started: 10,
    updated: 20,
    events: [],
  };
  const html = renderToStaticMarkup(
    <AgentActivityFeed
      events={[]}
      children={[child]}
      seconds={10}
      running={false}
    />,
  );
  expect(html).toContain('class="agent-child-activity"');
  expect(html).toContain("执行失败");
});
