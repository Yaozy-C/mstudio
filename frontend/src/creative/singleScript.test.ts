import { expect, test } from "bun:test";
import { newProject } from "../model";
import { createScript } from "./script";
import { applyOperations } from "../assistant/projectCommands";

test("manual creation preserves the existing project script", () => {
  const project = createScript(newProject("Single script"));
  expect(createScript(project)).toBe(project);
});

test("agent cannot add a second script, including in one batch", () => {
  const empty = newProject("Single script");
  const first = {
    op: "add_node",
    id: "script",
    kind: "screenplay",
    title: "脚本",
  };
  const second = { ...first, id: "duplicate" };
  expect(() => applyOperations(empty, [first, second])).toThrow("已有脚本");
  expect(empty.nodes).toHaveLength(0);
  const project = applyOperations(empty, [first]);
  expect(() => applyOperations(project, [second])).toThrow("已有脚本");
  const updated = applyOperations(project, [
    { op: "update_node", id: "script", title: "修改后的脚本" },
  ]);
  expect(updated.nodes[0].title).toBe("修改后的脚本");
  expect(updated.nodes[0].id).toBe("script");
});

test("screenplays reject retired contracts and summary fields atomically", () => {
  const empty = newProject("Structured screenplay");
  const base = {
    op: "add_node",
    id: "script",
    kind: "screenplay",
    title: "Script",
  };
  for (const operation of [
    { ...base, kind: "plan" },
    { ...base, text: "Promotional synopsis" },
    { ...base, screenplay: { story: "Global story" } },
    { ...base, screenplay: { sound: "Global sound" } },
  ]) {
    expect(() => applyOperations(empty, [operation])).toThrow();
    expect(empty.nodes).toHaveLength(0);
  }
});

test("a screenplay never creates a production canvas card", async () => {
  const { productionItems } = await import("../production/items");
  const project = createScript(newProject("Script workspace"));
  expect(project.nodes[0].kind).toBe("screenplay");
  expect(productionItems(project)).toEqual([]);
});
