import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { GeneralSettings } from "./GeneralSettings";
import { formatBytes } from "./storageUsageModel";

test("disk sizes use binary units and handle empty or invalid values", () => {
  expect(formatBytes(0)).toBe("0 B");
  expect(formatBytes(-10)).toBe("0 B");
  expect(formatBytes(NaN)).toBe("0 B");
  expect(formatBytes(1024)).toBe("1.0 KiB");
  expect(formatBytes(2.5 * 1024 ** 3)).toBe("2.5 GiB");
});
test("general settings expose separate display and storage sections without a fake cleanup in browser preview", () => {
  const html = renderToStaticMarkup(<GeneralSettings />);
  expect(html).toContain("settings-display");
  expect(html).toContain("settings-storage");
  expect(html).toContain("外观与显示");
  expect(html).toContain("请在桌面应用中管理素材目录");
  expect(html).not.toContain("确认清理");
});
