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
  expect(html).toContain("当前文件未修改");
  expect(html).toContain("错误详情");
  expect(html).toContain("error decoding response body");
  expect(html).toContain("请根据镜头说明创建预演");
  expect(html).not.toContain("原消息和附件");
  expect(html).not.toContain("&quot;code&quot;");
});
