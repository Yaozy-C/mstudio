import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { AnalysisProgress } from "./AnalysisProgress";
import { setLanguage } from "../../i18n";
afterEach(() => setLanguage("zh-CN"));

test("download bar shows measured percentage and bytes, while model analysis stays indeterminate", () => {
  const download = renderToStaticMarkup(
    <AnalysisProgress
      progress={{
        stage: "downloading",
        completed: 1024 ** 2,
        total: 4 * 1024 ** 2,
        startedAt: Date.now(),
      }}
      cancel={() => {}}
    />,
  );
  expect(download).toContain('aria-valuenow="25"');
  expect(download).toContain("1.0 MB / 4.0 MB");
  const streaming = renderToStaticMarkup(
    <AnalysisProgress
      progress={{
        stage: "downloading",
        completed: 2,
        total: 8,
        fragments: true,
        startedAt: Date.now(),
      }}
      cancel={() => {}}
    />,
  );
  expect(streaming).toContain("2 / 8 个片段");
  expect(streaming).toContain('aria-valuenow="25"');
  const model = renderToStaticMarkup(
    <AnalysisProgress
      progress={{ stage: "analyzing", startedAt: Date.now() - 46_000 }}
      cancel={() => {}}
    />,
  );
  expect(model).not.toContain("aria-valuenow=");
  expect(model).toContain('data-indeterminate="true"');
  expect(model).toContain("第 3 步，共 4 步");
  expect(model).toContain("模型仍在分析");
  expect(model.match(/data-state="done"/g)).toHaveLength(2);
});

test("unknown download size remains indeterminate and progress UI is translated", () => {
  setLanguage("en");
  const html = renderToStaticMarkup(
    <AnalysisProgress
      progress={{
        stage: "downloading",
        completed: 512,
        total: null,
        startedAt: Date.now(),
      }}
      cancel={() => {}}
    />,
  );
  expect(html).not.toContain("aria-valuenow=");
  expect(html).not.toMatch(/[\u3400-\u9fff]/);
  expect(html).toContain("Stop");
});
