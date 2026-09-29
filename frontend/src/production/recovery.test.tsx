import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { GenerationRun } from "./GenerationRun";
import type { ProductionController } from "./useProduction";
import type { ProductionTask } from "./types";
import { retryDelay, retryTask } from "./recovery";
import { fixture, model } from "./fixtures.test-helper";
import { saveTask } from "./document";
import {
  needsTracking,
  type GeneratedJob,
} from "../workspace/generatedJobTracking";

const task: ProductionTask = {
  key: "original",
  turnId: "turn",
  jobId: "old-job",
  requestId: "provider-id",
  status: "FAILED",
  kind: "video",
  mode: "multi",
  modelId: "model",
  inputs: [],
  prompt: "保持原镜头",
  parameters: { duration: 5, resolution: "480P" },
  error: "HTTP 422",
  createdAt: 1,
};
test("retry preserves task identity and settings but renews job identities", () => {
  const next = retryTask(task);
  const p = saveTask(saveTask(fixture(), task), next);
  expect(Object.values(p.production!.drafts!)).toHaveLength(1);
  expect(p.production!.drafts![task.key]).toBe(next);
  expect(next.key).toBe(task.key);
  expect(next.createdAt).toBe(task.createdAt);
  expect(next.turnId).toBe(task.turnId);
  expect(next.prompt).toBe(task.prompt);
  expect(next.parameters).toEqual(task.parameters);
  expect(next.inputs).not.toBe(task.inputs);
  expect(next.jobId).toBeUndefined();
  expect(next.requestId).toBeUndefined();
  expect(next.submissionId).toBeUndefined();
  expect(next.status).toBe("AWAITING_CONFIRMATION");
  for (const status of [
    "UNKNOWN",
    "IN_PROGRESS",
    "CANCEL_REQUESTED",
    "RECEIVING",
  ])
    expect(() => retryTask({ ...task, status })).toThrow();
});
test("temporary polling errors back off and eventually pause", () => {
  expect([1, 2, 3, 4].map(retryDelay)).toEqual([8000, 16000, 30000, undefined]);
});
const canvas = {
  media: { models: [model("minimax/h3/reference-to-video")] },
  items: [],
  modelPreferences: {},
} as unknown as ProductionController;
const render = (status: string) =>
  renderToStaticMarkup(
    <GenerationRun
      task={{ ...task, status }}
      project={fixture()}
      canvas={canvas}
      settings={() => {}}
      follow={() => {}}
    />,
  );
test("recovery actions match terminal, uncertain, cancelling and receiving states", () => {
  for (const status of ["FAILED", "CANCELLED"])
    expect(render(status)).toContain("重试</button>");
  for (const status of [
    "UNKNOWN",
    "CANCEL_REQUESTED",
    "IN_PROGRESS",
    "RECEIVING",
  ])
    expect(render(status)).not.toContain("重试</button>");
  expect(render("RECEIVING")).toContain("重试收取结果");
  expect(render("UNKNOWN")).toContain("核查后重新设置");
  expect(render("IN_PROGRESS")).toContain("进度暂时无法更新");
  expect(render("CANCEL_REQUESTED")).toContain("正在取消");
  expect(render("FAILED")).toContain("错误详情");
  expect(render("FAILED")).toContain("INVALID_INPUT");
  expect(
    needsTracking(
      { id: "old-job", status: "CANCEL_REQUESTED" } as GeneratedJob,
      fixture(),
    ),
  ).toBe(true);
});
