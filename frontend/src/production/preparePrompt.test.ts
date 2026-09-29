import { expect, test } from "bun:test";
import { preparePrompt } from "./preparePrompt";
import type { bridge } from "../bridge";
import type { ProductionTask } from "./types";

const draft: ProductionTask = {
  key: "draft",
  kind: "video",
  mode: "ends",
  modelId: "media",
  prompt: "调整上盖",
  ownerId: "old-shot",
  instruction: "旧要求",
  parameters: { duration: 8 },
  inputs: [
    {
      key: "s",
      nodeId: "script-node",
      assetId: "",
      role: "script",
      purpose: "历史脚本",
    },
    { key: "a", assetId: "a", role: "first-frame", purpose: "首帧" },
    { key: "b", assetId: "b", role: "last-frame", purpose: "尾帧" },
  ],
};
test("preparation sends only current prompt, parameters and explicit text references; preserves exact frame roles", async () => {
  let args: unknown;
  const invoke = (async (command: string, input: unknown) => {
    expect(command).toBe("prepare_media_prompt");
    expect((input as any).request.mediaModelId).toBe("media");
    args = input;
    return {
      prompt: "保留布面织纹，修正上盖网兜",
      references: [
        { assetId: "a", purpose: "起始画面，保持商品外观" },
        { assetId: "b", purpose: "结束画面，上盖网兜参考" },
      ],
    };
  }) as typeof bridge;
  const result = await preparePrompt(invoke, "project", draft);
  const serialized = JSON.stringify(args);
  expect(serialized).not.toContain("历史脚本");
  expect(serialized).not.toContain("旧要求");
  expect(serialized).not.toContain("old-shot");
  expect(serialized).toContain(
    '"contextReferences":[{"kind":"node","id":"script-node"}]',
  );
  expect(result.inputs.map((r) => r.role)).toEqual([
    "script",
    "first-frame",
    "last-frame",
  ]);
  expect(result.prompt).toContain("布面织纹");
  expect(result.parameters).toEqual({ duration: 8 });
  expect(draft.prompt).toBe("调整上盖");
});
test("failed or reordered preparation never yields an executable task", async () => {
  const failed = (async () => {
    throw new Error("连接失败");
  }) as typeof bridge;
  await expect(preparePrompt(failed, "p", draft)).rejects.toThrow("连接失败");
  const reordered = (async () => ({
    prompt: "test",
    references: [
      { assetId: "b", purpose: "test" },
      { assetId: "a", purpose: "test" },
    ],
  })) as typeof bridge;
  await expect(preparePrompt(reordered, "p", draft)).rejects.toThrow(
    "尚未提交",
  );
});

for (const kind of ["image", "video"] as const) {
  test(`${kind} preparation reads explicit documents while keeping them out of media uploads`, async () => {
    const invoke = (async (_: string, args: any) => {
      expect(args.request.contextReferences).toEqual([
        { kind: "asset", id: "brief" },
      ]);
      expect(args.request.inputs).toEqual([]);
      return { prompt: "依据文档整理后的画面描述", references: [] };
    }) as typeof bridge;
    const result = await preparePrompt(invoke, "project", {
      ...draft,
      kind,
      inputs: [
        { key: "brief", assetId: "brief", role: "script", purpose: "产品说明" },
      ],
    });
    expect(result.prompt).toBe("依据文档整理后的画面描述");
    expect(result.inputs[0].assetId).toBe("brief");
  });
}
