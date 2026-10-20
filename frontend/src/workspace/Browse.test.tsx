import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { existsSync } from "node:fs";
import { setLanguage } from "../i18n";
import { Browse } from "./Browse";
import { browseWorks } from "./browseWorks";
import { StudioSidebar } from "./StudioSidebar";

afterEach(() => setLanguage("zh-CN"));

test("browse is a watch-only gallery in both languages and does not preload films", () => {
  for (const language of ["zh-CN", "en"] as const) {
    setLanguage(language);
    const html = render(<Browse />);
    expect(html).toContain(
      language === "en" ? "External reference" : "外部参考作品",
    );
    expect(html).toContain(
      language === "en"
        ? "Watch the full film Coffee Run"
        : "观看完整作品 Coffee Run",
    );
    expect(html.match(/<article/g)).toHaveLength(3);
    expect(html).not.toMatch(/<(form|input|video|iframe)\b/);
    expect(html).not.toContain("新建项目");
    expect(html).not.toContain("Create project");
    if (language === "en") expect(html).not.toMatch(/[\u3400-\u9fff]/);
  }
});

test("an empty catalog never fabricates video cards", () => {
  const html = render(<Browse works={[]} />);
  expect(html).toContain("作品准备中");
  expect(html).not.toMatch(/<(img|video|article|button)\b/);
});

test("Browse is a distinct active navigation item, hidden in project-scoped settings", () => {
  const props = {
    onBrowse: () => {},
    onProjects: () => {},
    onSettings: () => {},
  };
  const html = render(
    <StudioSidebar {...props} activePage="browse" count={21} />,
  );
  expect(html).toMatch(/aria-current="page"[^>]*>[\s\S]*?浏览<\/button>/);
  expect(html.indexOf("浏览")).toBeLessThan(html.indexOf("项目空间"));
  const scoped = render(
    <StudioSidebar {...props} project={{ name: "Project" }} />,
  );
  expect(scoped).not.toContain("浏览");
  expect(scoped).toContain("返回项目");
});

test("catalog entries have actual local posters, full-film streams and attribution", () => {
  expect(new Set(browseWorks.map((work) => work.id)).size).toBe(
    browseWorks.length,
  );
  for (const work of browseWorks) {
    expect(new URL(work.video).origin).toBe("https://video.blender.org");
    expect(work.video).toEndWith(".mp4");
    expect(work.duration).toBeGreaterThan(180);
    expect(work.credit).toContain("Blender Foundation");
    expect(new URL(work.source).origin).toBe("https://studio.blender.org");
    expect(
      existsSync(
        new URL(`../../public/${work.poster.slice(2)}`, import.meta.url),
      ),
    ).toBe(true);
  }
});
