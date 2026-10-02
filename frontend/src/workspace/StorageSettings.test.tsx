import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { StorageSettings } from "./StorageSettings";
import { commandError } from "../errors/commands";
import { normalizeError } from "../errors/catalog";

test("browser storage is unavailable, not loading or failing", () => {
  const html = renderToStaticMarkup(<StorageSettings />);
  expect(html).toContain("请在桌面应用中管理素材目录");
  expect(html).not.toContain("正在读取");
  expect(html).not.toContain('role="alert"');
  expect(html).not.toContain("选择新目录并迁移");
});
test("storage read failures explain reading rather than migration", () => {
  const error = normalizeError(commandError("storage_settings", "read failed"));
  expect(error.code).toBe("STORAGE_READ_FAILED");
  expect(error.message).toBe("无法读取存储位置");
});
