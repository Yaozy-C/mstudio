import { test, expect } from "bun:test";
import { Runtime } from "./runtime";
test("Cordis unload removes native command access, even across queued toggles", async () => {
  const runtime = new Runtime(async <T>() => "ok" as T);
  await runtime.toggle("export", true);
  expect(await runtime.execute<string>("render_video", {})).toBe("ok");
  await Promise.all([
    runtime.toggle("export", false),
    runtime.toggle("export", true),
    runtime.toggle("export", false),
  ]);
  expect(runtime.enabled("export")).toBe(false);
  await expect(runtime.execute<string>("render_video", {})).rejects.toThrow(
    "运行模块尚未初始化",
  );
});
