import { expect, test } from "bun:test";
import type { ProductionTask } from "../production/types";
import { terminalOutputCount, type GeneratedJob } from "./generatedJobTracking";
import { runProgress } from "../production/runProgress";
const task = {
  key: "run",
  jobId: "job",
  status: "IN_PROGRESS",
} as ProductionTask;
const job = { id: "job", status: "IN_PROGRESS" } as GeneratedJob;
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

test("completed output count is required and never guessed from a single asset", () => {
  for (const outputCount of [undefined, 0, -1, 1.5])
    expect(() =>
      terminalOutputCount({
        ...job,
        status: "COMPLETED",
        outputCount,
        asset: { id: "one" },
      }),
    ).toThrow("实际输出数量");
  expect(
    terminalOutputCount({
      ...job,
      status: "COMPLETED",
      outputCount: 2,
      assets: [{ id: "one" }, null],
    }),
  ).toBe(2);
});
