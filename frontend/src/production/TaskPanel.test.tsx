import type { ContextType } from "react";
import { Context } from "../previs/context";
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

test("previs activity uses the existing toolbar task counts without a workspace overlay", () => {
  const canvas = {
    runs: [],
    taskPanelOpen: false,
  } as unknown as ProductionController;
  const html = renderToStaticMarkup(
    <Context.Provider
      value={
        {
          tasks: [
            {
              fileId: "running",
              name: "Shot",
              phase: "generating",
              running: true,
              error: "",
              stopping: false,
              lastKind: "chat",
            },
            {
              fileId: "failed",
              name: "Model",
              phase: "generating",
              running: false,
              error: "Synthetic failure",
              stopping: false,
              lastKind: "chat",
            },
          ],
        } as NonNullable<ContextType<typeof Context>>
      }
    >
      <TaskPanel
        project={newProject("Previs")}
        canvas={canvas}
        settings={() => {}}
      />
    </Context.Provider>,
  );
  expect(html).toContain("生成任务");
  expect(html).toContain("<span>2</span>");
  expect(html).toContain("1 个进行中");
  expect(html).toContain("1 个失败");
  expect(html).not.toContain("previs-background-task");
});
