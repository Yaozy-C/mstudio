import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ModelInputs } from "./ModelInputs";
import { newConnection } from "./types";
import { setLanguage } from "../i18n";

test("all input types remain selectable even for a known model", () => {
  setLanguage("zh-CN");
  const html = renderToStaticMarkup(
    <ModelInputs
      profile={{
        ...newConnection(),
        endpoint: "https://api.deepseek.com",
        model: "deepseek-flash",
        inputs: { image: true, audio: true, video: true, document: true },
      }}
      update={() => {}}
    />,
  );
  expect(html.match(/type="checkbox"/g)).toHaveLength(4);
  expect(html.match(/checked=""/g)).toHaveLength(4);
  expect(html).not.toContain("disabled");
  expect(html).not.toContain("<small");
  expect(html).not.toContain("<p");
  expect(html).not.toContain("<button");
});
