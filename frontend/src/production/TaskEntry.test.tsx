import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskEntry } from "./TaskEntry";
import { createTask } from "./tasks";
import { fixture } from "./fixtures.test-helper";
import type { ProductionController } from "./useProduction";

test("conversation entries display the same stored name for direct and Agent tasks", () => {
  const canvas = { showTask() {} } as unknown as ProductionController;
  const task = {
    ...createTask(fixture(), []),
    prompt: "Complete prompt",
    instruction: "Generate everything",
  };
  for (const turnId of ["direct:one", "agent-turn"])
    for (const title of ["小满 · 人物资产", "D01 · 分镜图", "D01 · 镜头视频"]) {
      const html = renderToStaticMarkup(
        <TaskEntry task={{ ...task, turnId, title }} canvas={canvas} />,
      );
      expect(html).toContain(`<strong>${title}</strong>`);
      expect(html).not.toContain("Generate everything");
    }
});
