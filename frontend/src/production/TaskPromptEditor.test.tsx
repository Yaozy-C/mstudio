import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskPromptEditor } from "./TaskPromptEditor";
import type { ProductionController } from "./useProduction";
import type { ProductionTask } from "./types";

test("pending and completed tasks show the same single prompt editor", () => {
  for (const status of ["AWAITING_CONFIRMATION", "IN_PROGRESS", "COMPLETED"]) {
    const task: ProductionTask = {
      key: "task",
      kind: "image",
      mode: "single",
      modelId: "model",
      prompt: "Product reference and scene",
      inputs: [],
      status,
    };
    const html = renderToStaticMarkup(
      <TaskPromptEditor task={task} canvas={{} as ProductionController} />,
    );
    expect((html.match(/<textarea/g) ?? []).length).toBe(1);
    expect((html.match(/<details/g) ?? []).length).toBe(1);
    expect(html).toContain("Product reference and scene");
    expect(html).not.toContain("下次");
    expect(html).not.toContain("原始");
  }
});
