import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { workflowAgent } from "./workflowAgents";
import { AgentWelcome } from "./AgentWelcome";
import { setLanguage } from "../i18n";

test("workspace suggests the responsible specialist instead of a mixed production menu", () => {
  setLanguage("zh-CN");
  expect(workflowAgent("concepts")).toBe("concept");
  expect(workflowAgent("script")).toBe("writer");
  expect(workflowAgent("assets")).toBe("assets");
  expect(workflowAgent("storyboard")).toBe("director");
  expect(workflowAgent("film")).toBe("editor");
  const asset = renderToStaticMarkup(
    <AgentWelcome
      view="assets"
      hasContent
      connected
      suggest={() => {}}
      connect={() => {}}
    />,
  );
  expect(asset).toContain("建立镜头资产");
  expect(asset).not.toContain("准备画面");
  expect(asset).not.toContain("打磨脚本");
  const writing = renderToStaticMarkup(
    <AgentWelcome
      view="script"
      hasContent
      connected
      suggest={() => {}}
      connect={() => {}}
    />,
  );
  expect(writing).toContain("打磨脚本");
  expect(writing).not.toContain("准备生成");
});
