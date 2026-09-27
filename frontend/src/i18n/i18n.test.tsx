import { afterEach, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import ts from "typescript";
import { setLanguage, getLanguage, resolveLanguage, t, LANGUAGE_KEY } from ".";
import messages from "./en.json";
import { LanguageSettings } from "../workspace/LanguageSettings";
import { StudioSidebar } from "../workspace/StudioSidebar";
import { ScriptNextStep } from "../creative/ScriptNextStep";
import { Theme } from "@radix-ui/themes";
import { ModelCenter } from "../models/ModelCenter";
import { ErrorNotice } from "../errors/ErrorNotice";

afterEach(() => setLanguage("zh-CN"));

test("saved choice wins; first launch follows the system with a safe fallback", () => {
  expect(resolveLanguage("en", "zh-CN")).toBe("en");
  expect(resolveLanguage("zh-CN", "en-US")).toBe("zh-CN");
  expect(resolveLanguage(null, "zh-TW")).toBe("zh-CN");
  expect(resolveLanguage("broken", "en-GB")).toBe("en");
});

test("language choice persists and storage failure still allows session switching", () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  const saved = new Map<string, string>();
  try {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        setItem: (key: string, value: string) => saved.set(key, value),
      },
    });
    setLanguage("en");
    expect(saved.get(LANGUAGE_KEY)).toBe("en");
    expect(resolveLanguage(saved.get(LANGUAGE_KEY)!, "zh-CN")).toBe("en");
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        setItem: () => {
          throw new Error("Storage blocked");
        },
      },
    });
    setLanguage("zh-CN");
    expect(getLanguage()).toBe("zh-CN");
  } finally {
    if (original) Object.defineProperty(globalThis, "localStorage", original);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
});

test("language UI, errors and dynamic messages switch in both directions", () => {
  setLanguage("en");
  expect(renderToStaticMarkup(<LanguageSettings />)).toContain(
    'aria-label="Interface language"',
  );
  expect(renderToStaticMarkup(<ErrorNotice error="HTTP 401" />)).toContain(
    "Service authentication failed",
  );
  expect(
    renderToStaticMarkup(
      <ScriptNextStep hasScript count={3} next={() => {}} />,
    ),
  ).toContain("3 shots planned");
  setLanguage("zh-CN");
  expect(renderToStaticMarkup(<LanguageSettings />)).toContain(
    'aria-label="界面语言"',
  );
  expect(t("任务编号：")).toBe("任务编号：");
});

test("project content remains verbatim even when it matches a translation key", () => {
  setLanguage("en");
  const html = renderToStaticMarkup(
    <StudioSidebar
      activePage="general"
      project={{ name: "新建项目" }}
      onProjects={() => {}}
      onSettings={() => {}}
    />,
  );
  expect(html).toContain("Back to project");
  expect(html).toContain('title="新建项目"');
  expect(t("未知的用户文本")).toBe("未知的用户文本");
  expect(t("constructor")).toBe("constructor");
  expect(t("当前界面语言：{language}", { language: "$& 中文 {raw}" })).toBe(
    "Current interface language: $& 中文 {raw}",
  );
});

test("all literal translation calls have English entries and matching placeholders", () => {
  const missing: string[] = [];
  function walk(directory: string) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) {
        walk(path);
        continue;
      }
      if (!/\.tsx?$/.test(path) || path.includes(".test.")) continue;
      const file = ts.createSourceFile(
        path,
        readFileSync(path, "utf8"),
        ts.ScriptTarget.Latest,
        true,
      );
      function visit(node: ts.Node) {
        if (
          ts.isCallExpression(node) &&
          ["t", "translate"].includes(node.expression.getText(file))
        ) {
          const key = node.arguments[0];
          if (
            key &&
            ts.isStringLiteral(key) &&
            /[\u3400-\u9fff]/.test(key.text) &&
            !(key.text in messages)
          )
            missing.push(`${path}: ${key.text}`);
        }
        ts.forEachChild(node, visit);
      }
      visit(file);
    }
  }
  walk(new URL("..", import.meta.url).pathname);
  expect(missing).toEqual([]);
  const placeholders = (value: string) =>
    [...value.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
  for (const [source, translation] of Object.entries(messages)) {
    expect(translation.trim().length).toBeGreaterThan(0);
    expect(placeholders(translation)).toEqual(placeholders(source));
  }
});

test("model category navigation renders in the selected language", () => {
  for (const language of ["en", "zh-CN", "en"] as const) {
    setLanguage(language);
    const html = renderToStaticMarkup(
      <Theme>
        <ModelCenter />
      </Theme>,
    );
    const navigation = html.match(/<nav[^>]*>[\s\S]*?<\/nav>/)?.[0];
    expect(navigation).toBeDefined();
    const labels =
      language === "en"
        ? ["Chat", "Images", "Video", "Service connections"]
        : ["对话", "图像", "视频", "服务连接"];
    for (const label of labels) expect(navigation).toContain(label);
    if (language === "en") expect(navigation).not.toMatch(/[\u3400-\u9fff]/);
  }
});
