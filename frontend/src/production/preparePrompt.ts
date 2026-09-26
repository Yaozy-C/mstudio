import type { ProductionTask } from "./types";
import type { bridge } from "../bridge";

export async function preparePrompt(
  invoke: typeof bridge,
  projectId: string,
  draft: ProductionTask,
): Promise<ProductionTask> {
  const inputs = draft.inputs.filter((r) => r.role !== "script");
  const prepared = await invoke<{
    prompt: string;
    references: { assetId: string; purpose: string }[];
  }>("prepare_media_prompt", {
    request: {
      projectId,
      kind: draft.kind,
      prompt: draft.prompt,
      parameters: draft.parameters ?? {},
      inputs: inputs.map(({ assetId, role, purpose, start, end }) => ({
        assetId,
        role,
        purpose,
        start,
        end,
      })),
    },
  });
  if (
    !prepared.prompt?.trim() ||
    prepared.references?.length !== inputs.length ||
    prepared.references.some(
      (r, i) => r.assetId !== inputs[i].assetId || !r.purpose?.trim(),
    )
  )
    throw new Error("提示词整理结果无效，尚未提交生成任务");
  return {
    ...draft,
    prompt: prepared.prompt,
    inputs: inputs.map((r, i) => ({
      ...r,
      purpose: prepared.references[i].purpose,
    })),
  };
}
