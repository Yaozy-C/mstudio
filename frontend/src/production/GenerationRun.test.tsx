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
      title: `Subject · ${kind}`,
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
    expect(html).toContain('class="task-generation-settings"');
    expect(html).toContain("生成模型");
    expect(html).toContain("任务名称");
    expect(html).toContain(`value="${task.title}"`);
    expect(html).toContain("添加参考素材");
    expect(html).not.toMatch(/>设置(?:<\/span>)?<\/button>/);
    expect(html).not.toContain("保存设置");
    if (kind === "video") expect(html).toContain("生成方式");
    expect(html).toMatch(/>重试(?:<\/span>)?<\/button>/);
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
