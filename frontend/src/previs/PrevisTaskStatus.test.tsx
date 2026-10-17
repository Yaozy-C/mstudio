import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Theme } from "@radix-ui/themes";
import { PrevisTaskStatus } from "./PrevisTaskStatus";
import { PrevisTaskList } from "./PrevisTaskList";
import { PrevisVisual } from "./PrevisVisual";
import { newModel } from "./types";
import type { PrevisEditorState } from "./usePrevisEditor";

function state(overrides: Partial<PrevisEditorState> = {}): PrevisEditorState {
  const file = { ...newModel("Bag"), kind: "shot" as const, revision: 99 };
  return {
    file,
    draft: file,
    project: { id: "p" },
    workspace: { files: [file] },
    phase: "rendering",
    lastKind: "render",
    busy: "render",
    error: "",
    stopping: false,
    frames: { done: 12, total: 24 },
    seconds: 8,
    models: [{ ...newModel("Reusable bag"), revision: 37 }],
    exports: [],
    runtime: { current: null },
    time: 0,
    ...overrides,
  } as unknown as PrevisEditorState;
}
test("render progress reports actual frames and background continuation", () => {
  const html = renderToStaticMarkup(<PrevisTaskStatus state={state()} stage />);
  expect(html).toContain('<progress value="12" max="24"');
  expect(html).toContain("正在渲染 12 / 24 帧");
  expect(html).toContain('data-state="done"');
  expect(html).toContain('data-state="active"');
  expect(html).toContain("后台继续");
});
test("background completion, failure and cancellation stay reviewable", () => {
  for (const overrides of [
    { busy: null, phase: "complete" },
    { busy: null, error: "Synthetic failure" },
    { busy: null, phase: "cancelled" },
  ] as Partial<PrevisEditorState>[]) {
    const html = renderToStaticMarkup(
      <Theme>
        <PrevisTaskList
          tasks={[
            { ...state(overrides), fileId: "bag", name: "Bag", running: false },
          ]}
          open={() => {}}
        />
      </Theme>,
    );
    expect(html).toContain("查看预演");
    expect(html).toContain("预演任务");
    expect(html).not.toContain("previs-background-task");
    expect(html).not.toContain("status-spinner");
  }
});
test("the editor treats a shared model as one editable file without a visible version", () => {
  const html = renderToStaticMarkup(
    <Theme>
      <PrevisVisual state={state({ busy: null, phase: null })} />
    </Theme>,
  );
  expect(html).toContain("Reusable bag");
  expect(html).not.toContain("v37");
  expect(html).not.toContain("v99");
  expect(html).not.toContain("已保存版本");
  expect(html).not.toContain("scene.js");
  expect(html).not.toContain("预演代码");
  expect(html).not.toContain("导入 JS 文件");
});
