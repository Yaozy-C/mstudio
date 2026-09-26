import { expect, test } from "bun:test";
import { newProject } from "../model";
import { createScript } from "./script";
import { applyOperations, inspectProject } from "../assistant/projectCommands";

test("manual creation preserves the existing project script", () => {
  const project = createScript(newProject("Single script"));
  expect(createScript(project)).toBe(project);
});

test("agent cannot add a second script, including in one batch", () => {
  const empty = newProject("Single script");
  const first = { op: "add_node", id: "script", kind: "plan", title: "脚本" };
  const second = { ...first, id: "duplicate" };
  expect(() => applyOperations(empty, 0, [first, second])).toThrow("已有脚本");
  expect(empty.nodes).toHaveLength(0);
  const project = applyOperations(empty, 0, [first]);
  expect(() =>
    applyOperations(project, inspectProject(project, {}).revision, [second]),
  ).toThrow("已有脚本");
  const updated = applyOperations(
    project,
    inspectProject(project, {}).revision,
    [{ op: "update_node", id: "script", title: "修改后的脚本" }],
  );
  expect(updated.nodes[0].title).toBe("修改后的脚本");
  expect(updated.nodes[0].id).toBe("script");
});
