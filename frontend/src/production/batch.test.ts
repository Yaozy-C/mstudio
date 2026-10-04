import { expect, test } from "bun:test";
import { newProject } from "../model";
import { batchSummary, changeBatch } from "./batch";
import { inspectProject } from "../assistant/inspectProject";
import { runsOf } from "./requestTask";
import type { ProductionTask } from "./types";
function fixture() {
  const p = newProject("100 video batch");
  const tasks: ProductionTask[] = Array.from({ length: 100 }, (_, i) => ({
    key: `run:${i}`,
    kind: "video",
    mode: "single",
    modelId: "video-model",
    inputs: [],
    prompt: "prompt ".repeat(1000),
    turnId: "batch",
    createdAt: i + 1,
    status: i < 60 ? "COMPLETED" : i < 80 ? "FAILED" : "AWAITING_CONFIRMATION",
    resultAssetId: i < 60 ? `asset-${i}` : undefined,
  }));
  p.production = {
    drafts: Object.fromEntries(tasks.map((task) => [task.key, task])),
  };
  return p;
}
test("durable batch summary and compact paging cover 100 tasks without losing any IDs", () => {
  const p = fixture();
  let offset = 0;
  const ids: string[] = [];
  do {
    const page = inspectProject(p, {
      section: "generation",
      turnId: "batch",
      fields: ["status"],
      offset,
    });
    expect(page.batch).toEqual({
      total: 100,
      counts: { COMPLETED: 60, FAILED: 20, AWAITING_CONFIRMATION: 20 },
      attention: 20,
    });
    // Thirty task states now include their actionable continuation, not prompts.
    expect(JSON.stringify(page).length).toBeLessThan(6000);
    expect(JSON.stringify(page)).not.toContain("prompt prompt");
    ids.push(...(page.items as { id: string }[]).map((task) => task.id));
    if (page.nextOffset === null) break;
    offset = page.nextOffset!;
  } while (true);
  expect(new Set(ids).size).toBe(100);
  const failed = inspectProject(p, {
    section: "generation",
    turnId: "batch",
    status: "FAILED",
    fields: ["status"],
  });
  expect(failed.total).toBe(20);
  expect(failed.batch?.total).toBe(100);
});
test("resume preserves completed jobs; retry resets only known failed tasks once, excluding partial outputs", () => {
  const p = fixture();
  p.production!.drafts!["run:60"].resultAssetId = "partial";
  p.production!.drafts!["run:61"].status = "UNKNOWN";
  const resumed = changeBatch(p, "batch", "resume");
  expect(batchSummary(runsOf(resumed)).counts.READY).toBe(20);
  const retried = changeBatch(resumed, "batch", "retry");
  expect(runsOf(retried)).toHaveLength(100);
  expect(changeBatch(retried, "batch", "retry")).toBe(retried);
  expect(
    runsOf(retried)
      .filter((task) => task.status === "READY")
      .every((task) => !task.jobId && !task.submissionId),
  ).toBe(true);
  const stopped = changeBatch(retried, "batch", "stop");
  expect(batchSummary(runsOf(stopped)).counts.CANCELLED).toBe(38);
  expect(batchSummary(runsOf(stopped)).counts.COMPLETED).toBe(60);
});
