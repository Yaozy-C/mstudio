import type { ProductionController } from "./useProduction";
export function TaskReference({ canvas }: { canvas: ProductionController }) {
  return (
    <>
      {canvas.referencedTask && (
        <div className="composer-task-reference">
          <button
            type="button"
            onClick={() => canvas.showTask(canvas.referencedTask!)}
          >
            已引用{canvas.referencedTask.kind === "image" ? "图片" : "视频"}
            任务 ·{" "}
            {(
              canvas.referencedTask.nextPrompt ?? canvas.referencedTask.prompt
            ).slice(0, 32)}
          </button>
          <button
            type="button"
            aria-label="移除任务引用"
            onClick={() => canvas.clearTaskReference()}
          >
            ×
          </button>
        </div>
      )}
    </>
  );
}
