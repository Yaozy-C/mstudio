import { expect, test } from "bun:test";
import type { Project } from "../model";
import { fixture } from "../production/fixtures.test-helper";
import type { ProductionTask } from "../production/types";
import { needsTracking, type GeneratedJob } from "./generatedJobTracking";
import { runProgress } from "../production/runProgress";
const task = {
  key: "run",
  jobId: "job",
  status: "IN_PROGRESS",
} as ProductionTask;
const project = (t = task) =>
  ({ ...fixture(), production: { drafts: { run: t } } }) as Project;
const job = { id: "job", status: "IN_PROGRESS" } as GeneratedJob;
test("durable jobs recover tracking without a localStorage or submit event", () => {
  expect(needsTracking(job, project())).toBe(true);
  expect(
    needsTracking(
      { ...job, status: "COMPLETED", asset: { id: "image" } },
      project(),
    ),
  ).toBe(true);
  expect(
    needsTracking(
      { ...job, status: "COMPLETED", asset: { id: "image" } },
      project({ ...task, status: "COMPLETED", resultAssetId: "image" }),
    ),
  ).toBe(false);
  expect(needsTracking({ ...job, status: "FAILED" }, project())).toBe(true);
  expect(
    needsTracking(
      { ...job, status: "FAILED" },
      project({ ...task, status: "FAILED" }),
    ),
  ).toBe(false);
});
test("stage presentation uses observed provider progress and prioritizes result import", () => {
  expect(
    runProgress({
      ...task,
      progress: { stage: "generating", message: "Codex 正在生成图片" },
    }),
  ).toBe("Codex 正在生成图片");
  expect(
    runProgress({
      ...task,
      status: "RECEIVING",
      progress: { stage: "generating", message: "old" },
    }),
  ).toContain("导入");
  expect(runProgress(task)).toContain("等待返回结果");
});
