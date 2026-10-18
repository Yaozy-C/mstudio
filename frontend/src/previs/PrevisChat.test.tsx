import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { commandError } from "../errors/commands";
import { PrevisChat } from "./PrevisChat";
import { newModel } from "./types";
import type { PrevisEditorState } from "./usePrevisEditor";

test("previs failures render a readable notice and retain a retryable prompt", () => {
  const error = commandError("previs_chat", "error decoding response body");
  const state = {
    file: newModel("Bag"),
    messages: [],
    busy: null,
    requestId: { current: "" },
    chatEnd: { current: null },
    error,
    prompt: "请根据镜头说明创建预演",
    catalog: { profiles: [] },
  } as unknown as PrevisEditorState;
  const html = renderToStaticMarkup(
    <Theme>
      <PrevisChat state={state} />
    </Theme>,
  );
  expect(html).toContain("预演生成未完成");
  expect(html).toContain("已完成的文件修改已保留");
  expect(html).toContain("错误详情");
  expect(html).toContain("error decoding response body");
  expect(html).toContain("请根据镜头说明创建预演");
  expect(html).not.toContain("原消息和附件");
  expect(html).not.toContain("&quot;code&quot;");
});

test("submitted requests replace the empty invitation with a visible task and active stop action", () => {
  const state = {
    file: newModel("Bag"),
    messages: [],
    busy: "chat",
    phase: "generating",
    lastKind: "chat",
    pendingPrompt: "打开包盖",
    prompt: "",
    frames: { done: 0, total: 0 },
    seconds: 7,
    stopping: false,
    chatEnd: { current: null },
    error: "",
    catalog: { profiles: [] },
  } as unknown as PrevisEditorState;
  const html = renderToStaticMarkup(
    <Theme>
      <PrevisChat state={state} />
    </Theme>,
  );
  expect(html).toContain("打开包盖");
  expect(html).toContain("正在生成预演场景…");
  expect(html).toContain("已用时 7 秒");
  expect(html).toContain("status-spinner");
  expect(html).toContain("可关闭此窗口，任务将在后台继续。");
  expect(html).not.toContain("描述模型的形状、尺寸和可动部件。");
  expect(html).not.toContain("正在修改代码");
  expect(html).toMatch(/<button[^>]*aria-label="停止"[^>]*aria-busy="false"/);
  expect(html).toContain("预演 Agent");
  expect(html).toContain("agent-composer");
  expect(html).toContain("agent-message user");
});
