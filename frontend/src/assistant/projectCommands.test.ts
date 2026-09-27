import { expect, test } from "bun:test";
import { newProject } from "../model";
import { applyOperations, inspectProject } from "./projectCommands";
test("agent command batch validates atomically, rejects stale edits and cannot import arbitrary files", () => {
  const p = newProject("film");
  expect(() =>
    applyOperations(p, 1, [{ op: "set_brief", text: "stale" }]),
  ).toThrow("工程已变化");
  expect(() =>
    applyOperations(p, 0, [
      { op: "add_node", id: "text", kind: "text", title: "script" },
      {
        op: "add_node",
        id: "bad",
        kind: "asset",
        title: "bad",
        assetId: "/secret",
      },
    ]),
  ).toThrow("已有素材");
  expect(p.nodes).toHaveLength(0);
  const next = applyOperations(p, 0, [
    { op: "add_node", id: "script", kind: "text", title: "剧本", text: "短片" },
  ]);
  expect(() => applyOperations(p, 0, [{ op: "connect" }])).toThrow();
  expect(inspectProject(next, { nodeIds: ["script"] }).details?.[0].text).toBe(
    "短片",
  );
});
test("Agent starts an empty project with ordinary cards and revises one without duplicating the rest", () => {
  const empty = newProject("旅行短片");
  const drafted = applyOperations(empty, inspectProject(empty, {}).revision, [
    {
      op: "set_creation",
      intent: "30 秒轻快旅行短片",
      preserve: "其他镜头不变",
    },
    {
      op: "add_node",
      id: "script",
      kind: "plan",
      title: "脚本",
      text: "出发、沿途、抵达",
    },
    {
      op: "add_node",
      id: "shot-1",
      kind: "shot",
      title: "01 · 出发",
      shot: { planId: "script", order: 1, duration: 5, dialogue: "" },
      text: "清晨推门出发",
    },
    {
      op: "add_node",
      id: "shot-2",
      kind: "shot",
      title: "02 · 沿途",
      shot: { planId: "script", order: 2, duration: 5, dialogue: "" },
      text: "午后海边行走",
    },
  ]);
  expect(empty.nodes).toHaveLength(0);
  expect(drafted.nodes.map((n) => n.kind)).toEqual(["plan", "shot", "shot"]);
  expect(drafted.nodes.every((n) => !n.resultAssetId)).toBe(true);
  expect(drafted.creation?.intent).toBe("30 秒轻快旅行短片");
  const revised = applyOperations(
    drafted,
    inspectProject(drafted, {}).revision,
    [{ op: "update_node", id: "shot-2", text: "傍晚海边行走" }],
  );
  expect(revised.nodes).toHaveLength(3);
  expect(revised.nodes.slice(0, 2)).toEqual(drafted.nodes.slice(0, 2));
  expect(revised.nodes[2]).toEqual({
    ...drafted.nodes[2],
    text: "傍晚海边行走",
    shot: {
      ...drafted.nodes[2].shot!,
      visualChanged: true,
    },
  });
  expect(revised.creation).toEqual(drafted.creation);
  expect(revised.assets).toHaveLength(0);
});

test("reorders and inserts shots atomically, validating only the final order", () => {
  const p = applyOperations(newProject("Reorder"), 0, [
    { op: "add_node", id: "plan", kind: "plan", title: "Plan" },
    ...Array.from({ length: 7 }, (_, i) => ({
      op: "add_node",
      id: `s${i + 1}`,
      kind: "shot",
      title: `Shot ${i + 1}`,
      shot: { planId: "plan", order: i + 1, duration: 2 },
    })),
  ]);
  const next = applyOperations(p, 0, [
    {
      op: "add_node",
      id: "cta",
      kind: "shot",
      title: "CTA",
      shot: { planId: "plan", order: 7, duration: 1.5 },
    },
    ...[4, 5, 6, 7].map((i) => ({
      op: "update_node",
      id: `s${i}`,
      shot: { order: i === 4 ? 8 : i - 1 },
    })),
  ]);
  expect(
    next.nodes
      .filter((n) => n.shot)
      .map((n) => n.shot!.order)
      .sort(),
  ).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
  expect(next.nodes.find((n) => n.id === "s4")?.shot?.order).toBe(8);
  expect(p.nodes.find((n) => n.id === "s4")?.shot?.order).toBe(4);
  expect(() =>
    applyOperations(p, 0, [
      { op: "update_node", id: "s4", shot: { order: 7 } },
    ]),
  ).toThrow("相同顺序");
  expect(p.nodes).toHaveLength(8);
});
