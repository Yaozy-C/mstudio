import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { commandError } from "../errors/commands";
import { PrevisChat } from "./PrevisChat";
import { newModel } from "./types";
import type { PrevisEditorState } from "./usePrevisEditor";
import { newConnection } from "../models/types";
import type { Asset } from "../model";

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

test("previs image attachments expose upload and project selection and block models without vision", () => {
  const model = {
    ...newConnection(),
    adapter: "codex" as const,
    name: "Codex",
  };
  const asset = {
    id: "image",
    name: "午餐包正面.png",
    kind: "image",
    path: "/assets/bag.png",
  } as Asset;
  const state = {
    file: newModel("Bag"),
    messages: [],
    busy: null,
    prompt: "参考这张图片建模",
    error: "",
    model,
    catalog: { profiles: [model] },
    project: { assets: [asset] },
    images: { items: [asset], busy: false, notice: "" },
  } as unknown as PrevisEditorState;
  const render = () =>
    renderToStaticMarkup(
      <Theme>
        <PrevisChat state={state} />
      </Theme>,
    );
  const blocked = render();
  expect(blocked).toContain('aria-label="上传参考图片"');
  expect(blocked).toContain('aria-label="引用项目图片"');
  expect(blocked).toContain('alt="午餐包正面.png"');
  expect(blocked).toContain('aria-label="移除参考图片 午餐包正面.png"');
  expect(blocked).toContain("当前模型不支持图片");
  expect(blocked).toMatch(/<button[^>]*aria-label="发送"[^>]*disabled/);
  model.inputs.image = true;
  const supported = render();
  expect(supported).not.toContain("当前模型不支持图片");
  expect(supported).not.toMatch(/<button[^>]*aria-label="发送"[^>]*disabled/);
  state.images.busy = true;
  expect(render()).toMatch(/<button[^>]*aria-label="发送"[^>]*disabled/);
});

test("submitted images move into the user message during generation and remain in saved history", () => {
  const image = { id: "image", name: "Bag.png", path: "/assets/bag.png" };
  const state = {
    file: newModel("Bag"),
    messages: [],
    busy: "chat",
    phase: "generating",
    pendingPrompt: "生成预演",
    pendingImages: [image],
    pendingRequestId: "request",
    prompt: "",
    error: "",
    catalog: { profiles: [] },
    images: { items: [], busy: false, notice: "" },
    project: { assets: [] },
    seconds: 0,
    frames: { done: 0, total: 0 },
  } as unknown as PrevisEditorState;
  const render = () =>
    renderToStaticMarkup(
      <Theme>
        <PrevisChat state={state} />
      </Theme>,
    );
  const pending = render();
  expect(pending).toContain('aria-label="已发送的参考图片"');
  expect(pending).toContain('alt="Bag.png"');
  expect(pending).not.toContain('aria-label="移除参考图片 Bag.png"');
  expect(pending).not.toContain('class="previs-image-strip"');
  expect(pending).toMatch(/<textarea[^>]*><\/textarea>/);
  state.messages = [
    {
      fileId: state.file.id,
      role: "user",
      text: "生成预演",
      images: [image],
      requestId: "request",
    },
  ];
  expect(render().match(/alt="Bag.png"/g)?.length).toBe(1);
  state.busy = null;
  state.phase = null;
  state.pendingPrompt = "";
  state.pendingImages = [];
  expect(render()).toContain('alt="Bag.png"');
});
