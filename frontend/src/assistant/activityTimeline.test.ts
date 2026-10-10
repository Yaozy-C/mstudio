import { expect, test } from "bun:test";
import { activityTimeline, withLiveThinking } from "./activityTimeline";
import type { ActivityEvent } from "./activityRows";
const event = (seq: number, kind: string, payload: unknown): ActivityEvent => ({
  seq,
  kind,
  payload,
  created: seq,
});

test("thinking, progress and tool operations stay interleaved after history reload", () => {
  const events = [
    event(1, "assistant/reasoning", { id: "a", text: "先读取参考" }),
    event(2, "assistant/progress", { text: "核对图片" }),
    event(3, "tool/call", { callId: "read", name: "mstudio_read_image" }),
    event(4, "tool/result", { callId: "read", result: { ok: true } }),
    event(5, "assistant/reasoning", { id: "b", text: "根据图片调整" }),
    event(6, "tool/call", {
      callId: "edit",
      name: "mstudio_edit",
      arguments: { operations: [{ op: "update_shot" }] },
    }),
  ];
  const rows = activityTimeline([...events].reverse());
  expect(rows.map((row) => row.kind)).toEqual([
    "thought",
    "progress",
    "tool",
    "thought",
    "tool",
  ]);
  expect(rows.map((row) => row.text || row.tool?.title)).toEqual([
    "先读取参考",
    "核对图片",
    "读取素材画面",
    "根据图片调整",
    "修改镜头",
  ]);
  expect(rows[2].tool?.detail).toBe("已完成");
  expect(rows[4].tool?.detail).toBe("执行中");
});

test("durable reasoning replaces its live snapshot without duplicating or losing the next block", () => {
  const live = [
    { id: "a", text: "先读" },
    { id: "b", text: "再检查" },
  ];
  const events = [
    event(1, "assistant/reasoning", { id: "a", text: "先读取参考" }),
    event(2, "tool/call", { callId: "read", name: "mstudio_read_image" }),
  ];
  expect(
    activityTimeline(withLiveThinking(events, live)).map(
      (row) => row.text || row.tool?.title,
    ),
  ).toEqual(["先读取参考", "读取素材画面", "再检查"]);
  expect(
    activityTimeline([event(1, "assistant/reasoning", { text: "  " })]),
  ).toEqual([]);
});
