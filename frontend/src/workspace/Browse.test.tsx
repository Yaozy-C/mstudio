import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup as render } from "react-dom/server";
import { setLanguage } from "../i18n";
import { Browse } from "./Browse";
import { browseWorks } from "./browseWorks";
import { StudioSidebar } from "./StudioSidebar";

afterEach(() => setLanguage("zh-CN"));

test("browse is a watch-only gallery in both languages and does not preload films", () => {
  for (const language of ["zh-CN", "en"] as const) {
    setLanguage(language);
    const html = render(<Browse />);
    expect(html).toContain(language === "en" ? "AI concept ad" : "AI 概念广告");
    expect(html).toContain(
      language === "en"
        ? "Watch the full film Mora Mora"
        : "观看完整作品 Mora Mora",
    );
    expect(html.match(/<article/g)).toHaveLength(6);
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

test("catalog has attributed streaming references, not copied or fake video assets", () => {
  expect(new Set(browseWorks.map((work) => work.id)).size).toBe(
    browseWorks.length,
  );
  for (const work of browseWorks) {
    const embed = new URL(work.embed);
    const poster = new URL(work.poster);
    expect(embed.origin).toBe("https://player.mux.com");
    expect(poster.origin).toBe("https://image.mux.com");
    expect(poster.pathname.split("/")[1]).toBe(embed.pathname.slice(1));
    expect(work.credit.length).toBeGreaterThan(2);
    expect(new URL(work.source).origin).toBe("https://runway.com");
  }
});
