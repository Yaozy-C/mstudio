import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { ServiceForm } from "./ServiceForm";
import { setLanguage } from "../i18n";

test("saved credentials use an empty replacement field and allow explicit removal", () => {
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
  expect(html).not.toContain("••••••••••••••••");
  expect(html).toContain("移除已保存密钥");
  expect(html).toContain('placeholder="已保存，留空保持原密钥" value=""');
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
