import { expect, test } from "bun:test";
import { newProject } from "../model";
import { contentChanged } from "./projectRevision";
import { applyOperations } from "../assistant/projectCommands";
import { restoreProject } from "./restoreProject";
import { jobStatus, saveTask } from "../production/document";
import { regenerationDraft } from "../production/taskEditing";
import type { ProductionTask } from "../production/types";

test("layout changes preserve content while real edits advance revisions", () => {
  const p = applyOperations(newProject("Revision"), [
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
  const edited = applyOperations(layout, [
    { op: "update_node", id: "note", title: "Edited" },
  ]);
  expect(edited.nodes[0].x).toBe(500);
  expect(contentChanged(layout, edited)).toBe(true);
  const current = { ...edited, revision: 5 };

  expect(restoreProject(p, layout).revision).toBe(4);
  expect(restoreProject(p, current).revision).toBe(6);
  expect(contentChanged(p, { ...p, clips: [...p.clips] })).toBe(false);
  expect(contentChanged(p, { ...p, brief: "new requirement" })).toBe(true);
});

test("background image/video progress does not invalidate edits or overwrite live task state", () => {
  for (const kind of ["image", "video"] as const) {
    let p = applyOperations(newProject("Concurrent progress"), [
      { op: "add_node", id: "note", kind: "note", title: "Original" },
    ]);
    const task: ProductionTask = {
      key: "run",
      kind,
      mode: "single",
      prompt: "Keep subject",
      inputs: [],
      modelId: "model",
      jobId: "job",
      status: "IN_QUEUE",
    };
    p = { ...saveTask(p, task), revision: 7 };
    let current = p;
    for (let i = 0; i < 20; i++) {
      const next = jobStatus(
        current,
        "job",
        "IN_PROGRESS",
        undefined,
        { stage: "generating", message: `Progress ${i}`, updatedAt: i },
        "remote",
      );
      current = {
        ...next,
        revision: current.revision! + Number(contentChanged(current, next)),
      };
    }
    expect(current.revision).toBe(7);
    const edited = applyOperations(current, [
      { op: "update_node", id: "note", title: "Corrected" },
    ]);
    expect(edited.nodes[0].title).toBe("Corrected");
    expect(edited.production!.drafts!.run.progress?.updatedAt).toBe(19);
    expect(edited.production!.drafts!.run.requestId).toBe("remote");
    expect(contentChanged(current, edited)).toBe(true);
    expect(() => regenerationDraft(current.production!.drafts!.run)).toThrow();

    for (const patch of [
      { status: "CANCEL_REQUESTED", trackingPaused: true, error: "offline" },
      { jobId: "submitted", submissionId: "submitted", requestId: "remote-2" },
    ])
      expect(contentChanged(p, saveTask(p, { ...task, ...patch }))).toBe(false);
    for (const patch of [
      { prompt: "User corrected material" },
      { modelId: "another-model" },
      { parameters: { resolution: "1080P" } },
      {
        inputs: [
          {
            key: "ref",
            assetId: "another-asset",
            role: "reference" as const,
            purpose: "subject",
          },
        ],
      },
      { resultAssetIds: ["new-result"] },
    ])
      expect(contentChanged(p, saveTask(p, { ...task, ...patch }))).toBe(true);
    expect(contentChanged(p, saveTask(p, { ...task, key: "new-run" }))).toBe(
      true,
    );
    expect(contentChanged(p, { ...p, production: { drafts: {} } })).toBe(true);
  }
});
