import { Theme } from "@radix-ui/themes";
import type { ReactNode } from "react";
import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { setLanguage } from "../../i18n";
import { VideoAnalysis } from "./VideoAnalysis";
import { createAnalysisController } from "./controller";
import { StudioSidebar } from "../StudioSidebar";
import { Browse } from "../Browse";
import { browseWorks } from "../browseWorks";
const render = (content: ReactNode) =>
  renderToStaticMarkup(<Theme>{content}</Theme>);
afterEach(() => setLanguage("zh-CN"));
const controller = () =>
  createAnalysisController(async <T,>() => undefined as T);

test("standalone page offers video sources in both languages without a project form", () => {
  for (const language of ["zh-CN", "en"] as const) {
    setLanguage(language);
    const service = controller();
    const html = render(
      <VideoAnalysis
        state={service.getSnapshot()}
        controller={service}
        onBrowse={() => {}}
        onSettings={() => {}}
      />,
    );
    expect(html).toContain(
      language === "en" ? "Choose video file" : "选择视频文件",
    );
    expect(html).not.toContain("项目名称");
    expect(html).not.toContain("Project name");
    expect(html).not.toContain("create_project");
    expect(html).not.toMatch(/<(video|iframe)\b/);
    if (language === "en") expect(html).not.toMatch(/[\u3400-\u9fff]/);
  }
});

test("catalog handoff keeps the full player, source credit and analysis action", () => {
  const service = controller();
  service.selectWork(browseWorks[0]);
  const html = render(
    <VideoAnalysis
      state={service.getSnapshot()}
      controller={service}
      onBrowse={() => {}}
      onSettings={() => {}}
    />,
  );
  expect(html).toContain(browseWorks[0].embed);
  expect(html).toContain(browseWorks[0].credit);
  expect(html).toContain("开始拆解");
  const gallery = render(<Browse onAnalyze={() => {}} />);
  expect(gallery.match(/拆解这部作品/g)).toHaveLength(6);
});

test("sidebar exposes standalone analysis as active, including return from settings", () => {
  const html = render(
    <StudioSidebar
      activePage="video-analysis"
      onProjects={() => {}}
      onBrowse={() => {}}
      onVideoAnalysis={() => {}}
      onSettings={() => {}}
    />,
  );
  expect(html).toMatch(/aria-current="page"[^>]*>[\s\S]*?视频拆解<\/button>/);
  const project = render(
    <StudioSidebar
      project={{ name: "Project" }}
      onProjects={() => {}}
      onVideoAnalysis={() => {}}
      onSettings={() => {}}
    />,
  );
  expect(project).not.toContain("视频拆解");
});
