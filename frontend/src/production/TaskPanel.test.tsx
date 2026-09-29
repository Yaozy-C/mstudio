import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskPanel } from "./TaskPanel";
import { newProject } from "../model";
import type { ProductionController } from "./useProduction";

test("task list remains discoverable before the first successful submission", () => {
  for (const taskPanelOpen of [false, true]) {
    const canvas = {
      runs: [],
      taskPanelOpen,
    } as unknown as ProductionController;
    const html = renderToStaticMarkup(
      <TaskPanel
        project={newProject("Empty")}
        canvas={canvas}
        settings={() => {}}
      />,
    );
    expect(html).toContain('aria-haspopup="dialog"');
    expect(html).toContain("生成任务");
    expect(html).not.toContain("production-task-panel");
    expect(html).toContain(`aria-expanded="${taskPanelOpen}"`);
  }
});
