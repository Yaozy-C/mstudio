import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ActivityEntries } from "./ActivityEntries";
import {
  ConversationState,
  type ConversationStates,
} from "./conversationState";
import type { ActivityEvent } from "./activityRows";

function fixture() {
  let serialized = 0;
  const events: ActivityEvent[] = [
    {
      seq: 1,
      created: 10,
      kind: "tool/call",
      payload: {
        callId: "read",
        name: "mstudio_read_image",
        arguments: {
          toJSON() {
            serialized++;
            return { assetId: "image-1", note: "<script>" };
          },
        },
      },
    },
    {
      seq: 2,
      created: 11,
      kind: "tool/result",
      payload: { callId: "read", result: { ok: true } },
    },
  ];
  return { events, serialized: () => serialized };
}

test("collapsed tool details do not serialize arguments or create hidden bodies", () => {
  const { events, serialized } = fixture();
  const html = renderToStaticMarkup(<ActivityEntries events={events} />);
  expect(html).toContain("读取素材画面");
  expect(html).toContain("已完成");
  expect(html).not.toContain("<pre>");
  expect(html).not.toContain("agent-activity-item-body");
  expect(serialized()).toBe(0);
});

test("restored open details render complete, escaped arguments and the latest result", () => {
  const { events, serialized } = fixture();
  const states: ConversationStates = new Map([
    ["message", new Map([["activity:1", true]])],
  ]);
  const render = () =>
    renderToStaticMarkup(
      <ConversationState id="message" states={states}>
        <ActivityEntries events={events} />
      </ConversationState>,
    );
  const html = render();
  expect(html).toContain('open=""');
  expect(html).toContain("调用参数");
  expect(html).toContain("image-1");
  expect(html).toContain("&lt;script&gt;");
  expect(html).toContain("返回结果");
  expect(html).toContain("true");
  expect(serialized()).toBe(1);
  events[1].payload = { callId: "read", result: { updated: "latest" } };
  expect(render()).toContain("latest");
});

test("disclosure state is isolated by message even when local event ids match", () => {
  const { events } = fixture();
  const states: ConversationStates = new Map([
    ["first", new Map([["activity:1", true]])],
  ]);
  const html = renderToStaticMarkup(
    <ConversationState id="second" states={states}>
      <ActivityEntries events={events} />
    </ConversationState>,
  );
  expect(html).not.toContain("<pre>");
});
