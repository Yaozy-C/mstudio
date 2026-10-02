import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { ServiceForm } from "./ServiceForm";
import { setLanguage } from "../i18n";

test("saved credentials render masked without a removal control", () => {
  setLanguage("zh-CN");
  const html = renderToStaticMarkup(
    <Theme>
      <ServiceForm
        initial={{
          id: "test",
          name: "fal",
          kind: "fal",
          endpoint: "https://queue.fal.run",
          hasKey: true,
          modelCount: 1,
        }}
        kinds={["fal"]}
        existing
        saved={() => {}}
        cancel={() => {}}
      />
    </Theme>,
  );
  expect(html).toContain('value="••••••••••••••••"');
  expect(html).not.toContain('type="checkbox"');
  expect(html).not.toContain("留空保持原密钥");
});
test("Codex connection has no endpoint or credential input", () => {
  setLanguage("zh-CN");
  const html = renderToStaticMarkup(
    <Theme>
      <ServiceForm
        initial={{
          id: "test",
          name: "Codex",
          kind: "codex",
          endpoint: "codex://local",
          hasKey: false,
          modelCount: 0,
        }}
        kinds={["codex"]}
        existing
        saved={() => {}}
        cancel={() => {}}
      />
    </Theme>,
  );
  expect(html).not.toContain("<input");
  expect(html).toContain("一键连接");
});
