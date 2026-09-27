import { expect, test } from "bun:test";
import { newProject } from "../model";
import { contentChanged } from "./projectRevision";
import { applyOperations } from "../assistant/projectCommands";
import { restoreProject } from "./restoreProject";

test("layout changes keep content edits valid, while real edits reject stale revisions", () => {
  const p = applyOperations(newProject("Revision"), 0, [
    { op: "add_node", id: "note", kind: "note", title: "Original" },
  ]);
  p.revision = 4;
  const layout = {
    ...p,
    viewport: { x: 10, y: 20, scale: 2 },
    nodes: p.nodes.map((n) => ({ ...n, x: 500, width: 600 })),
    production: { positions: { note: { x: 30, y: 40 } } },
  };
  expect(contentChanged(p, layout)).toBe(false);
  const edited = applyOperations(layout, 4, [
    { op: "update_node", id: "note", title: "Edited" },
  ]);
  expect(edited.nodes[0].x).toBe(500);
  expect(contentChanged(layout, edited)).toBe(true);
  const current = { ...edited, revision: 5 };
  expect(() =>
    applyOperations(current, 4, [
      { op: "update_node", id: "note", title: "Overwrite" },
    ]),
  ).toThrow("工程已变化");
  expect(restoreProject(p, layout).revision).toBe(4);
  expect(restoreProject(p, current).revision).toBe(6);
  expect(contentChanged(p, { ...p, clips: [...p.clips] })).toBe(false);
  expect(contentChanged(p, { ...p, brief: "new requirement" })).toBe(true);
});
