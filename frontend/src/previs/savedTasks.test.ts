import { expect, test } from "bun:test";
import { latestPrevisVideos, savedPrevisTasks } from "./savedTasks";
import { emptyWorkspace, newModel, type PrevisExport } from "./types";
import type { PrevisTaskSummary } from "./PrevisTaskList";

const video = (fileId: string, id = fileId): PrevisExport => ({
  id,
  fileId,
  fileRevision: 1,
  modelRevisions: {},
  signature: id,
  asset: {
    id: `asset-${id}`,
    name: `Video ${id}`,
    path: `/${id}.mp4`,
    preview: "",
    kind: "video",
    duration: 5,
    width: 720,
    height: 1280,
    hasAudio: false,
  },
  savedAssetId: null,
});

test("reopening a project restores saved renders without any in-memory tasks", () => {
  const workspace = {
    ...emptyWorkspace,
    files: [{ ...newModel("Shot one"), id: "s1", kind: "shot" as const }],
    exports: [video("s1"), video("s2")],
  };
  const result = savedPrevisTasks(workspace, []);
  expect(result).toHaveLength(2);
  expect(result[0]).toMatchObject({
    fileId: "s1",
    name: "Shot one",
    phase: "complete",
    running: false,
    lastKind: "render",
  });
  expect(result[1].name).toBe("Video s2");
  expect(result.every((task) => !task.error && !task.running)).toBe(true);
});

test("a retry or failure overrides an old saved result without duplicating the task", () => {
  const workspace = { ...emptyWorkspace, exports: [video("s1")] };
  const task: PrevisTaskSummary = {
    fileId: "s1",
    name: "Shot one",
    phase: "rendering",
    running: true,
    stopping: false,
    error: "",
    lastKind: "render",
  };
  expect(savedPrevisTasks(workspace, [task])).toEqual([task]);
  const failed = { ...task, running: false, error: "Encoding failed" };
  expect(savedPrevisTasks(workspace, [failed])).toEqual([failed]);
  // Merely opening the editor publishes no phase and must not hide a saved video.
  expect(savedPrevisTasks(workspace, [{ ...task, phase: null }])[0].phase).toBe(
    "complete",
  );
});

test("old export stacks show the latest render per shot, preserving orphaned results", () => {
  const exports = [video("s1", "new"), video("s2"), video("s1", "old")];
  expect(latestPrevisVideos(exports).map((entry) => entry.id)).toEqual([
    "new",
    "s2",
  ]);
  expect(savedPrevisTasks({ ...emptyWorkspace, exports }, [])).toHaveLength(2);
  expect(exports).toHaveLength(3);
});

test("empty projects and removed transient sessions do not invent completed renders", () => {
  const task: PrevisTaskSummary = {
    fileId: "gone",
    name: "Removed",
    phase: "complete",
    running: false,
    stopping: false,
    error: "",
    lastKind: "chat",
  };
  expect(savedPrevisTasks(emptyWorkspace, [task])).toEqual([]);
});
