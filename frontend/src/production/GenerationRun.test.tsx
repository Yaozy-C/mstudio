import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { GenerationRun } from "./GenerationRun";
import { fixture } from "./fixtures.test-helper";
import { regenerationDraft } from "./taskEditing";
import { runProgress } from "./runProgress";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";

test("image and video tasks retain settings after cancellation and reset execution before retry", () => {
  for (const kind of ["image", "video"] as const) {
    const task: ProductionTask = {
      key: kind,
      kind,
      mode: "single",
      prompt: "Subject",
      inputs: [],
      modelId: "old-model",
      parameters: { resolution: "720P" },
      status: "CANCELLED",
      jobId: "old-job",
      submissionId: "old-job",
      requestId: "remote",
    };
    const canvas = { media: { models: [] } } as unknown as ProductionController;
    const html = renderToStaticMarkup(
      <GenerationRun
        task={task}
        project={fixture()}
        canvas={canvas}
        settings={() => {}}
        follow={() => {}}
      />,
    );
    expect(html).toContain("已取消");
    expect(html).toContain(">设置</button>");
    expect(html).toContain(">重试</button>");
    expect(html).not.toContain("重新查询状态");
    const draft = regenerationDraft(task);
    expect(draft.status).toBe("AWAITING_CONFIRMATION");
    expect(draft.jobId).toBeUndefined();
    expect(draft.submissionId).toBeUndefined();
    expect(draft.requestId).toBeUndefined();
    expect(draft.parameters).toEqual(task.parameters);
  }
});

test("paused cancellation does not claim active polling or allow premature regeneration", () => {
  const task: ProductionTask = {
    key: "pending",
    kind: "video",
    mode: "single",
    prompt: "Subject",
    inputs: [],
    modelId: "m",
    status: "CANCEL_REQUESTED",
    trackingPaused: true,
  };
  expect(runProgress(task)).toBe("取消结果核查已暂停");
  expect(() => regenerationDraft(task)).toThrow();
});
